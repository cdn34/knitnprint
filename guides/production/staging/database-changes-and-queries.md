# Staging database changes and queries

## Explanation

Yes—**the one-off Fargate migration task is the right approach for our staging schema changes.** The database itself runs in RDS; Fargate runs the migration program and then exits.

Our existing [migration guide](pocket-guides/database-migration.md) covers the process:

1. Add a new SQL file under `migrations/` and test locally.
2. Take an RDS snapshot.
3. Build the backend image containing the migration.
4. Run the dedicated Fargate migration task and verify its exit code and logs.
5. Deploy the updated application.

Use migrations for tables, columns, indexes, and repeatable data changes required by a release. Ordinary business-data edits should usually go through the admin application.

**For querying the database**, our current configuration uses private RDS PostgreSQL, so your laptop needs a connection into the VPC. Two practical options are:

| Option | Best use |
|---|---|
| Temporary Fargate task containing `psql` | Occasional SQL scripts or diagnostics |
| Systems Manager tunnel through a small EC2 instance | Interactive access from `psql`, DBeaver, or another desktop database client |

The tunnel keeps RDS private and lets you use a familiar database interface. It requires setting up the intermediary instance, permissions, and database network access. [AWS connection guide](https://aws.amazon.com/blogs/database/securely-connect-to-an-amazon-rds-or-amazon-ec2-database-instance-remotely-with-your-preferred-gui/)

**The RDS console’s SQL Query Editor doesn’t support our ordinary RDS PostgreSQL instance.** It supports eligible Aurora clusters with Data API enabled. [AWS Query Editor documentation](https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/query-editor.html)

For us, I’d keep the existing migration workflow and add a Systems Manager tunnel with a read-only database user for routine inspection. Neither interactive access option is currently configured in the repository.

## Before running commands

The explanation above is preserved from the discussion. The steps below describe
changes for you to perform; adding this guide does not provision a tunnel or change
the database. Run shell commands from the repository root. Examples target only
`knitnprint-staging`, account `739863594156`, region `eu-west-1`.

Have AWS CLI v2, Docker with Buildx, Rust, Node/npm, jq, and the Terraform version
required by `infrastructure/terraform/staging/application/versions.tf` installed.
Use the configured Terraform backend and existing staging state. Authenticate:

```bash
aws sso login --profile knitnprint-administrator
aws sts get-caller-identity --profile knitnprint-administrator
terraform -chdir=infrastructure/terraform/staging/application init
```

Confirm the account ID before continuing. Stop on a failed command. Do not commit
credentials, Terraform plans, or SQL query output containing customer information.

## 1. Add a migration and test locally

Find the latest migration number:

```bash
ls migrations | sort | tail -5
```

At the time of writing the latest is `0051_branded_order_emails.sql`. Choose the
next unused number when you actually make your change. For example, an additive
column migration would be:

```bash
test ! -e migrations/0052_orders_internal_reference.sql &&
cat > migrations/0052_orders_internal_reference.sql <<'SQL'
ALTER TABLE orders ADD COLUMN internal_reference text;
SQL
```

This is an example change, not a migration you need for database access. Replace
the filename and SQL with your intended change. Keep deployed migration files
unchanged: SQLx records their checksums. For a data backfill, use a new migration
with a specific predicate; for a large backfill, use a separately reviewed,
bounded job. Keep changes compatible with the old API until rollout completes.

```bash
docker compose up -d
set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint
npm run db:migrate
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run api:check
git diff --check
git diff -- migrations/
```

Exercise the changed application flow locally too. Commit your actual migration
and any related application changes before building an image, for example:

```bash
git add -- migrations/0052_orders_internal_reference.sql
git commit -m "feat(db): add internal order reference"
```

## 2–5. Snapshot, build, run the migration, and deploy

Follow [the migration release checklist](pocket-guides/database-migration.md),
sections 2–12, in order. It contains the exact staging commands for each step:

| Requested step | Checklist sections | What to verify |
|---|---|---|
| 2. Take an RDS snapshot | 2–3 | Correct account; snapshot becomes `available` |
| 3. Build the backend image | 4–5 | `linux/amd64`; push to ECR; pin immutable digest |
| 4. Run the migration task | 6–10 | Register migration definition only; run once; exit `0` and success log |
| 5. Deploy the application | 11–12 | Review a fresh full plan; service stable; health and readiness pass |

The complete commands from that checklist are reproduced below for convenience.
If it changes, use the linked checklist as the release source of truth. A snapshot
restore creates another RDS instance; it is not an automatic in-place undo.

### 2. Authenticate and set release values

```bash
aws sso login --profile knitnprint-administrator

TF_ROOT='infrastructure/terraform/staging/application'
RELEASE_SHA="$(git rev-parse HEAD)"
API_REPOSITORY='739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-api'

printf 'Release: %s\n' "${RELEASE_SHA}"
```

### 3. Snapshot RDS

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

- [ ] Status is `available`.
- [ ] Record `SNAPSHOT_ID`. A snapshot restore creates a separate RDS instance.

### 4. Build and inspect the backend image

```bash
docker buildx version
docker buildx ls

docker buildx build \
  --platform linux/amd64 \
  --file backend/Dockerfile \
  --tag "${API_REPOSITORY}:${RELEASE_SHA}" \
  --load \
  .

docker image inspect "${API_REPOSITORY}:${RELEASE_SHA}" \
  --format '{{.Os}}/{{.Architecture}} {{json .Config.Entrypoint}} {{json .Config.Cmd}} {{.Config.User}}'
```

- [ ] Output is `linux/amd64 null ["/usr/local/bin/knitnprint-api"] 65532:65532`.

### 5. Push and obtain the ECR digest

```bash
aws ecr get-login-password \
  --profile knitnprint-administrator \
  --region eu-west-1 \
| docker login \
  --username AWS \
  --password-stdin 739863594156.dkr.ecr.eu-west-1.amazonaws.com

docker push "${API_REPOSITORY}:${RELEASE_SHA}"

API_DIGEST="$(
  aws ecr describe-images \
    --repository-name knitnprint-staging-api \
    --image-ids "imageTag=${RELEASE_SHA}" \
    --profile knitnprint-administrator \
    --region eu-west-1 \
    --query 'imageDetails[0].imageDigest' \
    --output text \
    --no-cli-pager
)"

printf '%s\n' "${API_DIGEST}"
```

- [ ] Digest is `sha256:` followed by 64 hexadecimal characters.
- [ ] Set `api_image_digest` in `${TF_ROOT}/variables.tf` to this digest.
- [ ] Commit the digest pin. The image tag remains the source commit SHA.

### 6. Register only the new migration task revision

```bash
terraform -chdir="${TF_ROOT}" fmt -check
terraform -chdir="${TF_ROOT}" validate
terraform -chdir="${TF_ROOT}" plan \
  -target=aws_ecs_task_definition.database_migration \
  -out=staging-migration-task.tfplan
terraform -chdir="${TF_ROOT}" show \
  -no-color staging-migration-task.tfplan
sha256sum "${TF_ROOT}/staging-migration-task.tfplan"
```

- [ ] The only managed-resource change is the migration task definition.
- [ ] Its image changes to `${API_DIGEST}`.
- [ ] The running ECS service, RDS, network, load balancer, S3, and secrets do not change.

```bash
terraform -chdir="${TF_ROOT}" apply staging-migration-task.tfplan
```

The `-target` warning is expected. This apply registers a task definition; it
does not run migrations or update the API service.

### 7. Resolve and inspect the active revision directly from ECS

Do not read this ARN from the aggregate Terraform output after a targeted apply.

```bash
MIGRATION_TASK_DEFINITION="$(
  aws ecs describe-task-definition \
    --task-definition knitnprint-staging-database-migration \
    --profile knitnprint-administrator \
    --region eu-west-1 \
    --query 'taskDefinition.taskDefinitionArn' \
    --output text \
    --no-cli-pager
)"

printf '%s\n' "${MIGRATION_TASK_DEFINITION}"

aws ecs describe-task-definition \
  --task-definition "${MIGRATION_TASK_DEFINITION}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query 'taskDefinition.{arn:taskDefinitionArn,status:status,revision:revision,containers:containerDefinitions[].{name:name,image:image,command:command,log:logConfiguration.options}}' \
  --output json \
  --no-cli-pager
```

- [ ] Status is `ACTIVE`.
- [ ] Image ends in `${API_DIGEST}`.
- [ ] Command is `/usr/local/bin/migrate`.
- [ ] Log group is `/aws/ecs/knitnprint-staging/api`, prefix `migration`.

### 8. Build the Fargate network configuration

```bash
PUBLIC_SUBNET_ID="$(terraform -chdir="${TF_ROOT}" output -json public_subnet_ids | jq -r '.[0]')"
APPLICATION_SG="$(terraform -chdir="${TF_ROOT}" output -json security_group_ids | jq -r '.application')"
NETWORK_CONFIGURATION="$(
  jq -nc \
    --arg subnet "${PUBLIC_SUBNET_ID}" \
    --arg security_group "${APPLICATION_SG}" \
    '{awsvpcConfiguration:{subnets:[$subnet],securityGroups:[$security_group],assignPublicIp:"ENABLED"}}'
)"

printf '%s\n' "${NETWORK_CONFIGURATION}" | jq .
```

- [ ] Values came from current Terraform outputs, not copied IDs.

### 9. Run the one-off migration task

```bash
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
```

### 10. Verify the migration before rolling the API

```bash
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

- [ ] Container exit code is `0`.
- [ ] Logs contain `database migrations applied`.
- [ ] Stop here if either check fails. Do not deploy the new API.

### 11. Generate a fresh full rollout plan

Never reuse a full plan created before the targeted apply.

```bash
terraform -chdir="${TF_ROOT}" plan \
  -out=staging-backend-rollout.tfplan
terraform -chdir="${TF_ROOT}" show \
  -no-color staging-backend-rollout.tfplan
sha256sum "${TF_ROOT}/staging-backend-rollout.tfplan"
```

- [ ] The migration task definition is not replaced again.
- [ ] Application and related API-image task definitions use `${API_DIGEST}`.
- [ ] The ECS service and worker schedule update as expected.
- [ ] No unexpected RDS, network, S3, CloudFront, secret, or load-balancer changes.

### 12. Apply and verify the backend rollout

```bash
terraform -chdir="${TF_ROOT}" apply staging-backend-rollout.tfplan

aws ecs wait services-stable \
  --cluster knitnprint-staging \
  --services knitnprint-staging \
  --profile knitnprint-administrator \
  --region eu-west-1

aws ecs describe-services \
  --cluster knitnprint-staging \
  --services knitnprint-staging \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query 'services[0].{desired:desiredCount,running:runningCount,pending:pendingCount,taskDefinition:taskDefinition,rollouts:deployments[].rolloutState}' \
  --output json \
  --no-cli-pager

curl --fail-with-body --silent --show-error \
  https://staging.knitnprint.com/api/health
curl --fail-with-body --silent --show-error \
  https://staging.knitnprint.com/api/ready

aws logs tail /aws/ecs/knitnprint-staging/api \
  --since 15m \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --format short \
  --no-cli-pager

terraform -chdir="${TF_ROOT}" plan
```

- [ ] Desired and running task counts match; pending is `0`.
- [ ] Health and readiness checks succeed.
- [ ] The affected application flow works.
- [ ] Final Terraform plan says `No changes`.

### Failure and rollback

If the migration task exits nonzero:

- Do not apply the full backend rollout.
- Preserve the task ARN and inspect its CloudWatch logs.
- Correct the migration, commit it, build and push a new image, pin its new
  digest, and repeat this procedure with a new targeted plan.
- Do not edit a migration that has already succeeded in staging; add a forward
  fix instead.

If the API rollout fails after a successful migration, pin `api_image_digest`
back to the previous known-good digest and apply a newly generated and reviewed
plan. The database migration is separate: prefer a compatible forward fix.
Restoring the RDS snapshot creates a separate database instance and is not an
automatic in-place rollback.

## Query using a one-off Fargate task

For routine interactive queries, use [the read-only tunnel guide](database-read-only-tunnel.md).
For an occasional diagnostic script, the existing bootstrap task contains `psql`.
The example below overrides its command, so it does **not** execute bootstrap SQL.
It uses the task's existing master credentials: it is an administrator operation,
not the read-only account. `BEGIN READ ONLY` limits this diagnostic transaction;
the underlying administrator login still has write privileges.

First inspect the task definition and ensure its container is named
`database-bootstrap` and its image is the pinned PostgreSQL image:

```bash
export AWS_PROFILE=knitnprint-administrator
export AWS_REGION=eu-west-1
export AWS_PAGER=''
TF_ROOT=infrastructure/terraform/staging/application
aws ecs describe-task-definition \
  --task-definition knitnprint-staging-database-bootstrap \
  --query 'taskDefinition.{containers:containerDefinitions[].{name:name,image:image,command:command},executionRole:executionRoleArn}'

SUBNET_ID="$(terraform -chdir="$TF_ROOT" output -json public_subnet_ids | jq -er '.[0]')"
APPLICATION_SG="$(terraform -chdir="$TF_ROOT" output -json security_group_ids | jq -er '.application')"
NETWORK_CONFIGURATION="$(jq -nc --arg subnet "$SUBNET_ID" --arg sg "$APPLICATION_SG" \
  '{awsvpcConfiguration:{subnets:[$subnet],securityGroups:[$sg],assignPublicIp:"ENABLED"}}')"

QUERY_OVERRIDES="$(jq -nc --arg script '
export PGPASSWORD="$MASTER_PASSWORD"
export PGSSLMODE=require
psql -X --host "$DATABASE_HOST" --port "$DATABASE_PORT" \
  --dbname "$DATABASE_NAME" --username "$MASTER_USERNAME" \
  --set ON_ERROR_STOP=1 --command "BEGIN READ ONLY; SELECT version, description, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 10; COMMIT;"
' '{containerOverrides:[{name:"database-bootstrap",command:["sh","-ec",$script]}]}')"

QUERY_RESULT="$(aws ecs run-task --cluster knitnprint-staging \
  --task-definition knitnprint-staging-database-bootstrap \
  --launch-type FARGATE --platform-version 1.4.0 --count 1 \
  --network-configuration "$NETWORK_CONFIGURATION" --overrides "$QUERY_OVERRIDES")"
printf '%s\n' "$QUERY_RESULT" | jq '{failures, tasks: [.tasks[] | {taskArn,lastStatus}]}'
QUERY_TASK_ARN="$(printf '%s\n' "$QUERY_RESULT" | jq -er '.tasks[0].taskArn')"
aws ecs wait tasks-stopped --cluster knitnprint-staging --tasks "$QUERY_TASK_ARN"
aws ecs describe-tasks --cluster knitnprint-staging --tasks "$QUERY_TASK_ARN" \
  --query 'tasks[0].{reason:stoppedReason,containers:containers[].{name:name,exitCode:exitCode,reason:reason}}'

BOOTSTRAP_LOG_GROUP="$(terraform -chdir="$TF_ROOT" output -json ecs_log_groups | jq -er '."database-bootstrap"')"
QUERY_TASK_ID="${QUERY_TASK_ARN##*/}"
aws logs get-log-events --log-group-name "$BOOTSTRAP_LOG_GROUP" \
  --log-stream-name "bootstrap/database-bootstrap/$QUERY_TASK_ID" \
  --query 'events[].message' --output text
```

Require no placement failures and container exit code `0`. This query emits only
migration metadata to CloudWatch. Do not use this pattern to dump customer tables
or secrets into logs. There is no interactive terminal in this one-off task.

## Query from your laptop or a desktop database client

Follow [Staging read-only database access through Systems Manager](database-read-only-tunnel.md).
It provides the Terraform to create the intermediary EC2 host, the database grants,
the tunnel command, verified-TLS `psql` connection, desktop-client settings, and
cleanup steps. RDS remains private throughout.

## Change business data through the admin application

Use `https://admin.staging.knitnprint.com` and the appropriate existing product,
order, or customer action. Verify the result through the storefront or read-only
SQL. There is no universal SQL replacement for these operations: direct updates
can skip validation, audit events, and notifications. If an operation is missing
from admin, implement that operation or write a reviewed repair script for the
specific data issue rather than changing arbitrary rows manually.
