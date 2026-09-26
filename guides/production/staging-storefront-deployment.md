# Deploy storefront changes to staging

The storefront is a server-rendered Node application inside the combined ECS
task. Each release is a Linux/AMD64 image in the private staging ECR repository,
pinned in Terraform by digest.

Staging runs one combined task, so a rollout can cause a short interruption.

## 1. Test and commit the release

Run from the repository root with a clean worktree:

```bash
npm ci
npm run typecheck --workspace=@knitnprint/storefront
npm run build --workspace=@knitnprint/storefront
npx playwright test tests/e2e/storefront.spec.ts --project=desktop-chromium
git status --short
```

Commit the intended files before building. ECR tags are immutable, and the full
commit SHA identifies the source used for the image.

## 2. Build and smoke-test the image

Confirm that Docker Buildx is available and that a builder is registered:

```bash
docker buildx version
docker buildx ls
```

```bash
RELEASE_SHA="$(git rev-parse HEAD)"
STOREFRONT_REPOSITORY='739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-storefront'

docker buildx build \
  --platform linux/amd64 \
  --file apps/storefront/Dockerfile \
  --tag "${STOREFRONT_REPOSITORY}:${RELEASE_SHA}" \
  --load \
  .

docker image inspect "${STOREFRONT_REPOSITORY}:${RELEASE_SHA}" \
  --format '{{.Os}}/{{.Architecture}} {{json .Config.Cmd}} {{.Config.User}}'
```

The output should start with `linux/amd64`, and the configured user should be
`knitnprint`. The build uses `--load` because the following steps inspect,
smoke-test, and push the image from the local Docker image store. For a fuller
local smoke test, use the storefront container commands in `local-setup.md`.

## 3. Push to private ECR

```bash
aws sso login --profile knitnprint-administrator

aws ecr get-login-password \
  --profile knitnprint-administrator \
  --region eu-west-1 \
| docker login \
  --username AWS \
  --password-stdin 739863594156.dkr.ecr.eu-west-1.amazonaws.com

docker push "${STOREFRONT_REPOSITORY}:${RELEASE_SHA}"
```

Read the registry digest; do not use a local image ID:

```bash
STOREFRONT_DIGEST="$(
  aws ecr describe-images \
    --repository-name knitnprint-staging-storefront \
    --image-ids "imageTag=${RELEASE_SHA}" \
    --profile knitnprint-administrator \
    --region eu-west-1 \
    --query 'imageDetails[0].imageDigest' \
    --output text \
    --no-cli-pager
)"

printf '%s\n' "${STOREFRONT_DIGEST}"
```

It must be a complete `sha256:<64 hex characters>` digest.

## 4. Pin and plan the rollout

Update the `storefront_image_digest` default in
`infrastructure/terraform/staging/application/variables.tf`, then commit that
pin. Do not deploy by changing an ECR tag in place.

```bash
terraform -chdir=infrastructure/terraform/staging/application fmt -check
terraform -chdir=infrastructure/terraform/staging/application validate
terraform -chdir=infrastructure/terraform/staging/application plan \
  -out=staging-storefront-rollout.tfplan
terraform -chdir=infrastructure/terraform/staging/application show \
  -no-color staging-storefront-rollout.tfplan
sha256sum infrastructure/terraform/staging/application/staging-storefront-rollout.tfplan
```

The normal plan replaces the application task-definition revision and updates
the ECS service in place. Confirm the API digest, desired count, secrets,
network, load balancer, and database are otherwise unchanged.

## 5. Apply and verify

```bash
terraform -chdir=infrastructure/terraform/staging/application apply \
  staging-storefront-rollout.tfplan

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
  https://staging.knitnprint.com/ >/dev/null
curl --fail-with-body --silent --show-error \
  https://staging.knitnprint.com/health
curl --fail-with-body --silent --show-error \
  https://staging.knitnprint.com/api/ready
curl --fail-with-body --silent --show-error \
  https://staging.knitnprint.com/robots.txt
curl --silent --show-error --head \
  https://staging.knitnprint.com/ | rg -i 'x-robots-tag|http/'

aws logs tail /aws/ecs/knitnprint-staging/storefront \
  --since 15m \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --format short \
  --no-cli-pager
```

Staging must continue returning its no-index controls.

## Roll back

Set `storefront_image_digest` back to the previous known-good ECR digest, create
and review a new saved plan, and apply it. Do not delete old ECR images during a
release.
