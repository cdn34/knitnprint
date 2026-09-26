# Pocket guide: staging database migration

Use this for a backend release containing new files under `migrations/`.

| Release record | Value |
|---|---|
| Operator / date | ______________________________ |
| Source commit | ______________________________ |
| RDS snapshot ID | ______________________________ |
| API digest | ______________________________ |
| Migration-plan SHA-256 | ______________________________ |
| Migration task ARN | ______________________________ |
| Full-plan SHA-256 | ______________________________ |

What happens:

1. The backend image is built with all migration files embedded in its
   `/usr/local/bin/migrate` executable and pushed to private ECR.
2. A targeted Terraform apply registers only a new migration task-definition
   revision. It does not run the migration or update the API service.
3. A one-off Fargate task pulls that image, connects to private RDS with the
   migration role, skips recorded migrations, and applies pending migrations
   in version order.
4. After exit code `0` and the success log are confirmed, a fresh full
   Terraform plan rolls the API and related workers to the same image.

Keep one terminal open for the entire procedure so its shell variables remain
available. Stop immediately when a command or checklist gate fails.

## 1. Validate locally

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
git status --short
```

- [ ] Local migrations and tests pass.
- [ ] New migrations are additive and compatible with the currently running API.
- [ ] No previously deployed migration was edited.
- [ ] Commit the intended source changes before building the image.

## 2. Authenticate and set release values

```bash
aws sso login --profile knitnprint-administrator

TF_ROOT='infrastructure/terraform/staging/application'
RELEASE_SHA="$(git rev-parse HEAD)"
API_REPOSITORY='739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-api'

printf 'Release: %s\n' "${RELEASE_SHA}"
```

## 3. Snapshot RDS

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

## 4. Build and inspect the backend image

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

## 5. Push and obtain the ECR digest

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

## 6. Register only the new migration task revision

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

## 7. Resolve and inspect the active revision directly from ECS

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

## 8. Build the Fargate network configuration

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

## 9. Run the one-off migration task

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

## 10. Verify the migration before rolling the API

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

## 11. Generate a fresh full rollout plan

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

## 12. Apply and verify the backend rollout

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

## Failure and rollback

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
