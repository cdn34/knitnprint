# Postmortem: stale task-definition output after targeted apply

- Date: 2026-09-26
- Environment: staging
- Status: resolved without data loss or additional service impact
- Affected workflow: migration-first backend deployment

## Summary

A targeted Terraform apply successfully registered revision `2` of the staging
database-migration ECS task definition with the new backend image. Terraform's
post-apply root output still reported revision `1`. The deployment was paused
before starting a migration task, and ECS was queried directly. AWS confirmed
that revision `2` was active and contained the reviewed image digest, migration
command, and CloudWatch log configuration.

The migration was launched explicitly with revision `2`. It logged `database
migrations applied` and exited `0`. The subsequent full Terraform plan updated
the stale root output from revision `1` to revision `2` while proposing no
additional migration-task replacement.

## Impact

No incorrect task was launched, no migration failed, and no RDS data was lost.
This incident caused no additional service impact beyond the storefront/API
version mismatch already under investigation. The running API continued using
its previous image during the investigation.

The risk was that following the stale Terraform output literally would have
started revision `1`, whose container referenced the old backend image. That
would not have applied the new embedded migrations and could have led to the
new API being rolled out against an incomplete schema.

## Architecture and intended sequence

The migration-first workflow deliberately uses a targeted apply:

```text
New backend image in ECR
        |
        v
Targeted Terraform apply registers only the new migration task definition
        |
        v
One-off Fargate task applies pending migrations to RDS
        |
        v
Fresh full Terraform plan rolls the API and workers to the new image
```

The targeted apply is intentional. Removing `-target` would update the running
API before its required migrations had completed. The defect was trusting an
aggregate Terraform output after a deliberately partial apply, not the use of
`-target` itself.

## Timeline

1. Backend image `adbe3edd034f74a8f2b64d34fe66b1777f534c35` was pushed to
   private ECR with OCI index digest
   `sha256:9da244543d3e6af94fbb855d952c5fa8f67391e306972a277116f83e761a762a`.
2. `api_image_digest` was pinned to that digest and committed.
3. A targeted saved plan proposed replacing only
   `aws_ecs_task_definition.database_migration`.
4. The reviewed targeted plan had SHA-256
   `b3e3caf894be95068591df3aacaa0c433e846bcf661270a47986668b95ccce34`
   and was applied successfully.
5. Terraform warned that `-target` could leave changes and outputs incomplete.
   Its displayed `ecs_task_definitions.database_migration` output still ended
   in revision `:1`.
6. Direct ECS queries showed that revision `:2` was active and referenced the
   new image digest.
7. Migration task
   `763ec97203e54159be872f4fa7f59818` was launched with revision `:2`.
8. CloudWatch logged `database migrations applied`; ECS reported exit code `0`
   with no container failure reason.
9. A fresh full plan proposed the expected three task-definition replacements
   and three in-place updates. It corrected the stale migration output from
   `:1` to `:2` without replacing the migration task again. Its SHA-256 was
   `38f2144997635f306dcedf75e0042e38fc18bff9a020cdbebeb6c714442e87d6`.

## Root cause

Terraform `-target` intentionally evaluates and applies a selected portion of
the dependency graph. Terraform warned that root outputs might not be fully
updated. The `ecs_task_definitions` output is an aggregate map containing
several task-definition resources, most of which were outside the targeted
graph. The selected migration resource changed successfully in Terraform state
and AWS, but the stored aggregate output remained stale until the next full
plan.

The runbook incorrectly treated that aggregate output as the authoritative
source for the migration task revision immediately after the targeted apply.

## Resolution

The deployment was paused before `ecs run-task`. The latest active task
definition was resolved from ECS, and its image, command, status, revision, and
log configuration were inspected. Only then was the migration task launched.

The staging database-operations guide now resolves the active migration task
definition directly from ECS after a targeted apply. It requires human
verification of the immutable image digest and command before launch.

## Verification commands used

### Review and apply the targeted plan

```bash
TF_ROOT='infrastructure/terraform/staging/application'

terraform -chdir="${TF_ROOT}" show \
  -no-color staging-migration-task.tfplan

sha256sum "${TF_ROOT}/staging-migration-task.tfplan"

terraform -chdir="${TF_ROOT}" apply \
  staging-migration-task.tfplan
```

### List active revisions directly from ECS

```bash
aws ecs list-task-definitions \
  --family-prefix knitnprint-staging-database-migration \
  --status ACTIVE \
  --sort DESC \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --output json \
  --no-cli-pager
```

The result contained revision `:2` as the active migration definition.

### Inspect the active revision and immutable image

```bash
aws ecs describe-task-definition \
  --task-definition knitnprint-staging-database-migration \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --query 'taskDefinition.{arn:taskDefinitionArn,status:status,revision:revision,containers:containerDefinitions[].{name:name,image:image,command:command,log:logConfiguration.options}}' \
  --output json \
  --no-cli-pager
```

Verified values:

```text
ARN:     arn:aws:ecs:eu-west-1:739863594156:task-definition/knitnprint-staging-database-migration:2
Status:  ACTIVE
Image:   739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-api@sha256:9da244543d3e6af94fbb855d952c5fa8f67391e306972a277116f83e761a762a
Command: /usr/local/bin/migrate
Logs:    /aws/ecs/knitnprint-staging/api with stream prefix migration
```

### Resolve the exact ARN used for launch

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
```

Expected suffix: `:2`.

### Verify network configuration

```bash
printf '%s\n' "${NETWORK_CONFIGURATION}" | jq .
```

Verified configuration:

```json
{
  "awsvpcConfiguration": {
    "subnets": ["subnet-002fa2e87423dbf6b"],
    "securityGroups": ["sg-0e6e9ce76ef5f6a4e"],
    "assignPublicIp": "ENABLED"
  }
}
```

### Launch the one-off Fargate migration task

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
```

### Wait for completion and inspect the exit code

```bash
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
```

Verified result:

```json
{
  "stopCode": "EssentialContainerExited",
  "stoppedReason": "Essential container in task exited",
  "containers": [
    {
      "name": "database-migration",
      "exitCode": 0,
      "reason": null
    }
  ]
}
```

`EssentialContainerExited` is normal for this one-off task because its
essential container is expected to finish. Exit code `0` is the success signal.

### Read the preserved migration logs

```bash
aws logs tail /aws/ecs/knitnprint-staging/api \
  --since 30m \
  --log-stream-name-prefix migration \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --format short \
  --no-cli-pager
```

Verified message:

```text
database migrations applied
```

### Regenerate and review the full rollout plan

```bash
terraform -chdir="${TF_ROOT}" plan \
  -out=staging-backend-rollout.tfplan

terraform -chdir="${TF_ROOT}" show \
  -no-color staging-backend-rollout.tfplan

sha256sum "${TF_ROOT}/staging-backend-rollout.tfplan"
```

The refreshed plan proposed `3 to add, 3 to change, 3 to destroy`, did not
replace the migration task definition again, and corrected the root output to
revision `:2`.

## Corrective actions

- Replaced the post-target Terraform-output lookup with an ECS
  `describe-task-definition` lookup.
- Added an explicit inspection gate for task status, revision, image digest,
  command, and log destination before `ecs run-task`.
- Documented that the targeted apply registers a task definition but does not
  execute it.
- Documented that a full plan created before the targeted apply becomes stale
  and must be regenerated after the migration succeeds.
- Retained `-target` for this controlled migration-first exception; removing it
  would introduce the more serious risk of rolling out the API before its
  schema is ready.
