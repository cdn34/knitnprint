# Deploy Rust backend changes to staging

The Rust API and its operational commands share one digest-pinned image in
private ECR. The API runs in the combined ECS application task. Scheduled and
one-off task definitions also use the same API digest.

If the release adds a database migration, read
[staging-database-operations.md](staging-database-operations.md) first and run
the backward-compatible migration before rolling the API service.

## 1. Test and commit

With the local PostgreSQL container running:

```bash
set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run api:check
git status --short
```

Commit the intended code before building so the immutable image tag identifies
the exact source.

## 2. Build the production image

Confirm that Docker Buildx is available and that a builder is registered:

```bash
docker buildx version
docker buildx ls
```

```bash
RELEASE_SHA="$(git rev-parse HEAD)"
API_REPOSITORY='739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-api'

docker buildx build \
  --platform linux/amd64 \
  --file backend/Dockerfile \
  --tag "${API_REPOSITORY}:${RELEASE_SHA}" \
  --load \
  .

docker image inspect "${API_REPOSITORY}:${RELEASE_SHA}" \
  --format '{{.Os}}/{{.Architecture}} {{json .Config.Entrypoint}} {{json .Config.Cmd}} {{.Config.User}}'
```

Expected image configuration:

```text
linux/amd64 null ["/usr/local/bin/knitnprint-api"] 65532:65532
```

The runtime is Google's distroless `nonroot` image. UID/GID `65532:65532` is
therefore intentional, and the image does not contain `/bin/sh`. A `null`
entrypoint is also expected because `backend/Dockerfile` configures the API as
the image's `CMD`; ECS replaces that command when it runs an operational binary
such as `migrate`.

The multi-stage `COPY` in `backend/Dockerfile` includes the API, migration
runner, and other operational binaries. The build fails if any required binary
is absent. Fully cached Buildx steps are valid because the cache is
content-addressed; still confirm that the final tag contains the current
`RELEASE_SHA`. The build uses `--load` because the following steps inspect and
push the image from the local Docker image store.

## 3. Push and obtain the immutable digest

```bash
aws sso login --profile knitnprint-administrator

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

Update the `api_image_digest` default in
`infrastructure/terraform/staging/application/variables.tf` and commit the pin.

## 4. Handle migrations before the service rollout

If there are new files under `migrations/`, stop here and follow the migration
sequence in [staging-database-operations.md](staging-database-operations.md).
Migrations must remain compatible with the currently running API until the
service rollout finishes.

If there are no schema changes, continue directly.

## 5. Plan and review the rollout

```bash
terraform -chdir=infrastructure/terraform/staging/application fmt -check
terraform -chdir=infrastructure/terraform/staging/application validate
terraform -chdir=infrastructure/terraform/staging/application plan \
  -out=staging-backend-rollout.tfplan
terraform -chdir=infrastructure/terraform/staging/application show \
  -no-color staging-backend-rollout.tfplan
sha256sum infrastructure/terraform/staging/application/staging-backend-rollout.tfplan
```

An API digest change can replace several task-definition revisions: the
application, migration, owner-bootstrap, and notification-worker definitions.
It should update the ECS service and scheduled worker target as needed. It must
not unexpectedly replace RDS, networking, S3, CloudFront, or the load balancer.

## 6. Apply and verify

```bash
terraform -chdir=infrastructure/terraform/staging/application apply \
  staging-backend-rollout.tfplan

aws ecs wait services-stable \
  --cluster knitnprint-staging \
  --services knitnprint-staging \
  --profile knitnprint-administrator \
  --region eu-west-1
```

```bash
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
```

Also test the affected API flow through the storefront or admin application.
Run a final Terraform plan and expect `No changes`.

## Roll back

Pin `api_image_digest` to the previous known-good ECR digest and apply a newly
reviewed plan. A schema rollback is separate and may not be safe; prefer
forward-compatible and forward-fix migrations.
