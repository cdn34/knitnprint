# Pocket guide: staging backend release

Use only when the release has no new database migrations. If `migrations/`
changed, stop: the migration-first procedure must register and run the new
migration task before the API service is updated.

| Release record | Value |
|---|---|
| Operator / date | ______________________________ |
| Source commit | ______________________________ |
| API digest | ______________________________ |
| Terraform-plan SHA-256 | ______________________________ |

This release builds the API image locally, pushes it to private ECR, pins its
immutable digest in Terraform, and updates the ECS/Fargate application and
operational task definitions. Keep one terminal open so its shell variables
remain available. Stop immediately when a command or checklist gate fails.

## 1. Test and commit

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

- [ ] Tests pass and `migrations/` has no release changes.
- [ ] Commit the intended source before building.

## 2. Build and inspect

```bash
RELEASE_SHA="$(git rev-parse HEAD)"
API_REPOSITORY='739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-api'

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

## 3. Push and pin the digest

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

- [ ] Set `api_image_digest` in `infrastructure/terraform/staging/application/variables.tf`.
- [ ] Commit the digest pin.

## 4. Plan and review

```bash
TF_ROOT='infrastructure/terraform/staging/application'

terraform -chdir="${TF_ROOT}" fmt -check
terraform -chdir="${TF_ROOT}" validate
terraform -chdir="${TF_ROOT}" plan \
  -out=staging-backend-rollout.tfplan
terraform -chdir="${TF_ROOT}" show \
  -no-color staging-backend-rollout.tfplan
sha256sum "${TF_ROOT}/staging-backend-rollout.tfplan"
```

- [ ] API-image task definitions move to `${API_DIGEST}`.
- [ ] ECS service and scheduled worker target update as expected.
- [ ] No unexpected RDS, network, S3, CloudFront, secret, or load-balancer changes.

## 5. Apply and verify

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

- [ ] Desired and running counts match; pending is `0`.
- [ ] Health, readiness, and the affected application flow succeed.
- [ ] Final Terraform plan says `No changes`.

## Roll back

Set `api_image_digest` in
`infrastructure/terraform/staging/application/variables.tf` to the previous
known-good ECR digest. Commit the pin, create and review a new saved Terraform
plan, apply it, wait for the ECS service to stabilize, and repeat every
verification in step 5. Do not reuse the original plan file.
