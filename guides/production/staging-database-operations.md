# Staging database migrations and operations

Staging PostgreSQL is private and has no SSH bastion. Normal access is through
the admin/API, reviewed one-off ECS tasks, and CloudWatch logs. Do not make RDS
public or copy database passwords into shell history, chat, Terraform, or local
files.

The database identities are intentionally separate:

- `knitnprint_migration` owns schema changes.
- `knitnprint_runtime` is used by the API and approved data-maintenance jobs.
- the RDS master credential is reserved for bootstrap and break-glass work.
- admin-area users are application identities, not PostgreSQL roles.

## Common task settings

Authenticate and derive current network/task values from Terraform rather than
copying old IDs from a previous deployment:

```bash
aws sso login --profile knitnprint-administrator

TF_ROOT='infrastructure/terraform/staging/application'
PUBLIC_SUBNET_ID="$(terraform -chdir="${TF_ROOT}" output -json public_subnet_ids | jq -r '.[0]')"
APPLICATION_SG="$(terraform -chdir="${TF_ROOT}" output -json security_group_ids | jq -r '.application')"
NETWORK_CONFIGURATION="$(
  jq -nc \
    --arg subnet "${PUBLIC_SUBNET_ID}" \
    --arg security_group "${APPLICATION_SG}" \
    '{awsvpcConfiguration:{subnets:[$subnet],securityGroups:[$security_group],assignPublicIp:"ENABLED"}}'
)"
```

Keep that terminal open for the commands below.

## Create and test a migration

Add the next immutable, sequentially numbered file under `migrations/`. Never
edit a migration that has already run in staging.

Test from an empty local database as well as the current development database:

```bash
docker compose up -d

set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint

npm run db:migrate
cargo test --workspace
```

Prefer additive, backward-compatible changes. The migration must work while
the previous API revision is still serving traffic. Split destructive column
drops or renames into a later release after old code is gone.

For a risky migration, create and wait for a manual RDS snapshot first:

```bash
SNAPSHOT_ID="knitnprint-staging-pre-migration-$(date -u +%Y%m%d%H%M%S)"

aws rds create-db-snapshot \
  --db-instance-identifier knitnprint-staging-postgres \
  --db-snapshot-identifier "${SNAPSHOT_ID}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager

aws rds wait db-snapshot-available \
  --db-snapshot-identifier "${SNAPSHOT_ID}" \
  --profile knitnprint-administrator \
  --region eu-west-1
```

Record the snapshot ID. Restoring it creates a separate database instance; it
is not an instant in-place undo.

## Publish the image and register the migration task first

Build and push the backend image using
[staging-backend-deployment.md](staging-backend-deployment.md), then update and
commit `api_image_digest` in `variables.tf`.

Before updating the running API, create a saved plan targeting only the
migration task definition:

```bash
terraform -chdir="${TF_ROOT}" plan \
  -target=aws_ecs_task_definition.database_migration \
  -out=staging-migration-task.tfplan
terraform -chdir="${TF_ROOT}" show -no-color staging-migration-task.tfplan
sha256sum "${TF_ROOT}/staging-migration-task.tfplan"
```

This targeted apply is a deliberate migration-first exception. Review that it
only registers the migration task revision, then apply it:

```bash
terraform -chdir="${TF_ROOT}" apply staging-migration-task.tfplan
```

Immediately after the migration succeeds, create and apply the full backend
rollout plan so Terraform is not left partially converged.

## Run the migration task

```bash
MIGRATION_TASK_DEFINITION="$(
  terraform -chdir="${TF_ROOT}" output -json ecs_task_definitions \
  | jq -r '.database_migration'
)"

MIGRATION_TASK_ARN="$(
  aws ecs run-task \
    --cluster knitnprint-staging \
    --task-definition "${MIGRATION_TASK_DEFINITION}" \
    --count 1 \
    --launch-type FARGATE \
    --platform-version 1.4.0 \
    --network-configuration "${NETWORK_CONFIGURATION}" \
    --profile knitnprint-administrator \
    --region eu-west-1 \
    --query 'tasks[0].taskArn' \
    --output text \
    --no-cli-pager
)"

printf '%s\n' "${MIGRATION_TASK_ARN}"

aws ecs wait tasks-stopped \
  --cluster knitnprint-staging \
  --tasks "${MIGRATION_TASK_ARN}" \
  --profile knitnprint-administrator \
  --region eu-west-1

aws ecs describe-tasks \
  --cluster knitnprint-staging \
  --tasks "${MIGRATION_TASK_ARN}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query 'tasks[0].{stopCode:stopCode,stoppedReason:stoppedReason,containers:containers[].{name:name,exitCode:exitCode,reason:reason}}' \
  --output json \
  --no-cli-pager

aws logs tail /aws/ecs/knitnprint-staging/api \
  --since 30m \
  --log-stream-name-prefix migration \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --format short \
  --no-cli-pager
```

Do not roll out the API unless the migration container exited `0` and logged
`database migrations applied`. Continue with the full plan in the backend
deployment guide.

## Routine operational checks

The existing scheduled-worker task already has the runtime database URL. It can
run the safe, compiled operational report without exposing credentials:

```bash
WORKER_TASK_DEFINITION="$(
  terraform -chdir="${TF_ROOT}" output -json ecs_task_definitions \
  | jq -r '.notification_worker'
)"

aws ecs run-task \
  --cluster knitnprint-staging \
  --task-definition "${WORKER_TASK_DEFINITION}" \
  --count 1 \
  --launch-type FARGATE \
  --platform-version 1.4.0 \
  --network-configuration "${NETWORK_CONFIGURATION}" \
  --overrides '{"containerOverrides":[{"name":"notification-worker","command":["/usr/local/bin/check_operations"]}]}' \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query '{taskArn:tasks[0].taskArn,failures:failures}' \
  --output json \
  --no-cli-pager
```

Wait for the returned task ARN as shown in the migration section, verify exit
code zero, then read:

```bash
aws logs tail /aws/ecs/knitnprint-staging/notification-worker \
  --since 15m \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --format short \
  --no-cli-pager
```

Use the admin application for ordinary orders, customers, products, inventory,
and operational state. That preserves authorization and audit behavior.

## Break-glass read-only SQL

When the admin/API cannot answer a diagnostic question, the existing PostgreSQL
bootstrap image can run a noninteractive `psql` query inside the VPC using the
restricted runtime role. The database transaction is forced read-only. The
bootstrap task definition also receives master bootstrap secrets even though
this command does not use them, so treat this as a break-glass path rather than
routine access. Query text and results are retained in AWS task metadata and
CloudWatch, so never select passwords, hashes, tokens, full addresses, payment
data, or other unnecessary personal data.

Set a bounded read-only query. Prefer IDs and counts:

```bash
STAGING_SQL="SELECT status, count(*) FROM notification_jobs GROUP BY status ORDER BY status"

QUERY_OVERRIDES="$(
  jq -nc \
    --arg sql "${STAGING_SQL}" \
    '{containerOverrides:[{
      name:"database-bootstrap",
      command:["sh","-ceu","export PGPASSWORD=\"$RUNTIME_PASSWORD\"; psql --host \"$DATABASE_HOST\" --port \"$DATABASE_PORT\" --dbname \"$DATABASE_NAME\" --username knitnprint_runtime --set ON_ERROR_STOP=1 --pset pager=off --command \"BEGIN TRANSACTION READ ONLY; $STAGING_SQL; COMMIT;\""],
      environment:[{name:"STAGING_SQL",value:$sql}]
    }]}'
)"

QUERY_TASK_DEFINITION="$(
  terraform -chdir="${TF_ROOT}" output -json ecs_task_definitions \
  | jq -r '.database_bootstrap'
)"

aws ecs run-task \
  --cluster knitnprint-staging \
  --task-definition "${QUERY_TASK_DEFINITION}" \
  --count 1 \
  --launch-type FARGATE \
  --platform-version 1.4.0 \
  --network-configuration "${NETWORK_CONFIGURATION}" \
  --overrides "${QUERY_OVERRIDES}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query '{taskArn:tasks[0].taskArn,failures:failures}' \
  --output json \
  --no-cli-pager

unset STAGING_SQL QUERY_OVERRIDES
```

Wait for and inspect the task, then read the last few minutes of
`/aws/ecs/knitnprint-staging/database-bootstrap`. Do not use this path for
writes. Repeated diagnostics should become a reviewed Rust operational command
instead.

## Customer account removal and anonymization

Do not run `DELETE FROM customers ... CASCADE`. Customer data spans accounts,
sessions, tokens, addresses, carts, orders, payments, fulfillment, media, and
audit records. Some commercial records may need to be retained and anonymized
rather than deleted.

The repository currently supports retention-based anonymization through
`cleanup_customers`. It removes expired authentication/address data,
anonymizes the customer record, and writes an audit entry. It does **not** offer
an immediate arbitrary-user erasure command.

Run the existing cleanup only for records whose `retention_expires_at` has
already passed:

```bash
aws ecs run-task \
  --cluster knitnprint-staging \
  --task-definition "${WORKER_TASK_DEFINITION}" \
  --count 1 \
  --launch-type FARGATE \
  --platform-version 1.4.0 \
  --network-configuration "${NETWORK_CONFIGURATION}" \
  --overrides '{"containerOverrides":[{"name":"notification-worker","command":["/usr/local/bin/cleanup_customers"],"environment":[{"name":"CUSTOMER_CLEANUP_BATCH_SIZE","value":"25"},{"name":"CUSTOMER_SESSION_RETENTION_DAYS","value":"7"}]}]}' \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query '{taskArn:tasks[0].taskArn,failures:failures}' \
  --output json \
  --no-cli-pager
```

For an immediate removal request, first implement a dedicated, tested Rust
command that accepts one customer ID, supports a dry run, performs one
transaction, revokes authentication, removes eligible personal data,
anonymizes records that must remain, handles owned media, and writes an audit
reason. Review that command against the schema and retention policy before
running it as a one-off ECS task. Until that command exists, use the admin UI
for inspection and do not improvise deletion SQL against staging.
