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

## ECS, Fargate, and CloudWatch in this stack

Amazon ECS is the container orchestrator and control plane. It stores task
definitions, starts tasks, and keeps services at their desired task count. A
task definition is an immutable, versioned blueprint describing container
images, commands, CPU and memory, secrets, networking, roles, and logging. An
ECS service maintains long-running tasks such as the API, while `ecs run-task`
starts a standalone task that may run once and stop.

AWS Fargate is compute capacity for ECS. ECS decides what should run from the
task definition; Fargate supplies and manages the machines on which those
containers run, so this repository does not provision an ECS worker fleet of
EC2 instances. Both the long-running application service and the one-off
migration task use Fargate.

Amazon ECR is the private container-image registry. It stores images already
built and pushed from the release workstation; neither ECS nor ECR builds the
images. ECS task definitions reference immutable ECR digests, and Fargate pulls
those images when ECS starts a task. The API, storefront, migration,
notification-worker, and other operational containers use these private
images. ClamAV also runs on Fargate but uses its separately pinned public image.
The static admin application is uploaded to private S3 and delivered through
CloudFront rather than run as a Fargate container.

CloudWatch Logs is the configured destination for container standard output
and standard error. `aws ecs run-task` is asynchronous and does not attach the
remote container to the local terminal. `ecs describe-tasks` reports lifecycle
state and the container exit code, while the `awslogs` driver preserves the
migration program's messages after its short-lived Fargate task stops.

```text
Docker Buildx -> image -> ECR
                           |
Terraform -> ECS task definition
                           |-> ECS service -> long-running Fargate task
                           `-> ecs run-task -> one-off Fargate migration task -> RDS
                                                  |
                                                  `-> CloudWatch Logs

Admin build -> private S3 -> CloudFront
```

See the AWS documentation for
[ECS task definitions](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task_definitions.html)
and
[Fargate task logging](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/fargate-tasks-services.html).

## How migration execution works

The backend image contains a dedicated `/usr/local/bin/migrate` executable.
`sqlx::migrate!` embeds the repository's migration files into that executable
when the image is built. If a migration file changes after the image build,
rebuild and republish the image before running the staging task.

The one-off ECS migration task runs inside the staging VPC and receives
`MIGRATION_DATABASE_URL` from Secrets Manager. It connects to private RDS as
the restricted `knitnprint_migration` database role; it does not use the RDS
master identity or the application's runtime identity.

For each run, SQLx:

1. Acquires a PostgreSQL advisory lock so only one migrator can run.
2. Creates `_sqlx_migrations` if it does not already exist.
3. Checks previously applied migration versions and checksums.
4. Walks every forward migration embedded in the binary in version order.
5. Skips versions already recorded as applied and executes only pending ones.
6. Records each successful version, checksum, and execution time.

Each normal migration file and its bookkeeping entry run in one transaction.
If a migration fails, that migration is rolled back, later migrations do not
run, and the task exits nonzero. Earlier migrations committed by the same run
remain applied. Do not roll out the new API until the task exits `0` and logs
`database migrations applied`.

Running `npm run db:migrate` with the local Docker PostgreSQL URL affects only
the local database. It verifies the migration set but does not change staging.
For example, if staging records versions 1 through 24 and the image embeds
versions 1 through 35, the ECS task validates and skips 1 through 24, then
applies 25 through 35 in order. The previous API revision continues serving
traffic during this migration-first step, so new migrations must remain
backward-compatible with it.

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

aws rds describe-db-snapshots \
  --db-snapshot-identifier "${SNAPSHOT_ID}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query 'DBSnapshots[0].Status' \
  --output text \
  --no-cli-pager
```

The final command must print `available`. Record the snapshot ID. Restoring it
creates a separate database instance; it is not an automatic or instant
in-place undo.

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

This `-target` is a deliberate migration-first exception. It lets Terraform
register a new revision of only the migration task definition even though the
same API digest will eventually update several other task definitions. ECS
task definitions are immutable, so Terraform replaces the managed resource by
registering a new revision and deregistering the old revision. It does not
modify RDS data, start a task, or update the running API service.

Read-only data sources may appear in the plan. The only managed resource change
must be `aws_ecs_task_definition.database_migration`, whose container image must
change to the reviewed API digest. The plan must not change the application
service or task definition, notification worker, scheduler, owner bootstrap,
RDS, networking, load balancer, S3, or secrets.

`terraform show` renders the saved binary plan for review. The checksum is an
audit record proving that the file applied later is byte-for-byte identical to
the file reviewed; Terraform and AWS do not consume the checksum. Do not commit
saved plan files because they can contain sensitive configuration.

Apply the reviewed targeted plan:

```bash
terraform -chdir="${TF_ROOT}" apply staging-migration-task.tfplan
```

The targeted apply only registers the task definition. It does not execute the
migration. At this temporary checkpoint, the running ECS service still uses
the old API image while the new migration task definition uses the new image:

```text
Running application service -> old API task definition and image
Migration task definition   -> new API image and embedded migrations
```

Launch and verify the one-off task in the next section. Immediately after it
succeeds, regenerate, review, and apply the full backend rollout plan so
Terraform is not left partially converged. Do not reuse a full plan created
before the targeted apply; the targeted apply changes Terraform state and
makes that earlier plan stale.

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
