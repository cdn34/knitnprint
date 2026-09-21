# Staging scheduled workers

Use this guide when adding another periodic backend command to staging. The
existing notification worker in
`infrastructure/terraform/staging/application/notification-worker.tf` is the
reference implementation.

The staging pattern is:

```text
EventBridge Scheduler -> one-shot ECS Fargate task -> Rust command -> exit
```

Do not add an always-running service for short periodic work. A worker must be
safe to retry, safe to run twice, and must terminate when its batch is done.

## 1. Implement and test the command

Choose names once, for example:

```bash
WORKER_COMMAND='cleanup_sessions'
WORKER_SLUG='cleanup-sessions'
```

Put the Rust entry point in `backend/src/bin/${WORKER_COMMAND}.rs`. Ensure
`backend/Dockerfile` copies the release binary into `/usr/local/bin`.

Test it against the disposable local database:

```bash
set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint

cargo fmt --all --check
cargo test --workspace
cargo run -p knitnprint-api --bin "${WORKER_COMMAND}"
```

The command should:

- claim bounded batches with database locking when multiple runs could overlap;
- record per-item failures and retries without duplicating completed work;
- return non-zero for invalid configuration or a failed database connection;
- avoid printing secrets or unnecessary personal data;
- finish well before the next scheduled invocation.

## 2. Package and publish the API image

Follow [staging-backend-deployment.md](staging-backend-deployment.md). Record the
immutable ECR digest and update `api_image_digest` in
`infrastructure/terraform/staging/application/variables.tf`.

## 3. Add the Terraform resources

Create a focused file such as `cleanup-workers.tf`. For each independently
scheduled command, define:

1. A 14-day CloudWatch log group by adding its name to `local.ecs_log_names` in
   `runtime-foundation.tf`.
2. An `aws_ecs_task_definition` using the digest-pinned API image, 256 CPU, and
   512 MB unless measurements justify more.
3. Only the required Secrets Manager values. Database jobs normally receive
   the runtime URL, never the RDS master credential.
4. A scheduler trust role and policy limited to `ecs:RunTask` for that task
   definition and `iam:PassRole` for its exact ECS roles.
5. An `aws_scheduler_schedule` with Fargate, the application security group,
   both public subnets, and `assign_public_ip = true`.
6. Terraform outputs for the task definition and schedule.

Reuse `aws_iam_role.api_task` only when the command needs the same S3/SES
permissions as the API. Otherwise create a smaller task role. Reusing the ECS
execution role is appropriate for image pulls, logs, and approved secret
injection.

For low-volume staging work, start with `rate(5 minutes)`. A one-minute schedule
creates 1,440 Fargate launches per day and is rarely justified for fewer than
five testers.

Make the schedule depend on its launch policy so the first invocation cannot
race IAM propagation:

```hcl
depends_on = [aws_iam_role_policy.<scheduler_policy>]
```

## 4. Validate, review, and apply

```bash
aws sso login --profile knitnprint-administrator

PLAN_FILE="staging-${WORKER_SLUG}.tfplan"

terraform -chdir=infrastructure/terraform/staging/application fmt -check
terraform -chdir=infrastructure/terraform/staging/application validate
terraform -chdir=infrastructure/terraform/staging/application plan \
  -out="${PLAN_FILE}"
terraform -chdir=infrastructure/terraform/staging/application show \
  -no-color "${PLAN_FILE}"
sha256sum "infrastructure/terraform/staging/application/${PLAN_FILE}"
```

Confirm that the plan changes only the worker, its logs, IAM, schedule, and any
intentional image-dependent task definitions. Never accept an unexplained
database, network, bucket, or load-balancer change.

Apply the reviewed plan:

```bash
terraform -chdir=infrastructure/terraform/staging/application apply \
  "${PLAN_FILE}"
```

## 5. Verify the first run

```bash
WORKER_NAME="knitnprint-staging-${WORKER_SLUG}"
WORKER_LOG_GROUP="/aws/ecs/knitnprint-staging/${WORKER_SLUG}"

aws scheduler get-schedule \
  --name "${WORKER_NAME}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager

aws ecs list-tasks \
  --cluster knitnprint-staging \
  --family "${WORKER_NAME}" \
  --desired-status STOPPED \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager
```

Copy the newest task ARN, then inspect it:

```bash
TASK_ARN='arn:aws:ecs:eu-west-1:739863594156:task/knitnprint-staging/<task-id>'

aws ecs describe-tasks \
  --cluster knitnprint-staging \
  --tasks "${TASK_ARN}" \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query 'tasks[0].{stopCode:stopCode,stoppedReason:stoppedReason,containers:containers[].{name:name,exitCode:exitCode,reason:reason}}' \
  --output json \
  --no-cli-pager

aws logs tail "${WORKER_LOG_GROUP}" \
  --since 15m \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --format short \
  --no-cli-pager
```

An ECS exit code of zero proves the command completed, but still inspect its
summary for per-item failures. Finally run an ordinary Terraform plan and
expect `No changes`.

## Pause or remove a worker

For a temporary pause, set the Terraform schedule state to `DISABLED`, review a
saved plan, and apply it. To remove a worker, review the destroy list carefully;
do not manually delete Terraform-managed AWS resources.
