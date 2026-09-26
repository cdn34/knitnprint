# Pocket guide: staging storefront release

| Release record | Value |
|---|---|
| Operator / date | ______________________________ |
| Source commit | ______________________________ |
| Storefront digest | ______________________________ |
| Terraform-plan SHA-256 | ______________________________ |

The storefront is a server-rendered Node container in the combined ECS task.
This release builds it locally, pushes it to private ECR, pins its immutable
digest in Terraform, and updates the ECS/Fargate service. Staging has one
combined task, so a rollout can cause a short interruption. Keep one terminal
open so its shell variables remain available. Stop immediately when a command
or checklist gate fails.

## 1. Test and commit

```bash
npm ci
npm run typecheck --workspace=@knitnprint/storefront
npm run build --workspace=@knitnprint/storefront
npx playwright test tests/e2e/storefront.spec.ts --project=desktop-chromium
git status --short
```

- [ ] Checks pass.
- [ ] Commit the intended source before building the image.

## 2. Build and inspect

```bash
RELEASE_SHA="$(git rev-parse HEAD)"
STOREFRONT_REPOSITORY='739863594156.dkr.ecr.eu-west-1.amazonaws.com/knitnprint-staging-storefront'

docker buildx version
docker buildx ls
docker buildx build \
  --platform linux/amd64 \
  --file apps/storefront/Dockerfile \
  --tag "${STOREFRONT_REPOSITORY}:${RELEASE_SHA}" \
  --load \
  .

docker image inspect "${STOREFRONT_REPOSITORY}:${RELEASE_SHA}" \
  --format '{{.Os}}/{{.Architecture}} {{json .Config.Cmd}} {{.Config.User}}'
```

- [ ] Platform is `linux/amd64`.
- [ ] Command is `["node",".output/server/index.mjs"]`.
- [ ] User is `knitnprint`.

## 3. Push and pin the digest

```bash
aws sso login --profile knitnprint-administrator

aws ecr get-login-password \
  --profile knitnprint-administrator \
  --region eu-west-1 \
| docker login \
  --username AWS \
  --password-stdin 739863594156.dkr.ecr.eu-west-1.amazonaws.com

docker push "${STOREFRONT_REPOSITORY}:${RELEASE_SHA}"

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

- [ ] Set `storefront_image_digest` in `infrastructure/terraform/staging/application/variables.tf`.
- [ ] Commit the digest pin.

## 4. Plan and review

```bash
TF_ROOT='infrastructure/terraform/staging/application'

terraform -chdir="${TF_ROOT}" fmt -check
terraform -chdir="${TF_ROOT}" validate
terraform -chdir="${TF_ROOT}" plan \
  -out=staging-storefront-rollout.tfplan
terraform -chdir="${TF_ROOT}" show \
  -no-color staging-storefront-rollout.tfplan
sha256sum "${TF_ROOT}/staging-storefront-rollout.tfplan"
```

- [ ] Application task definition moves to `${STOREFRONT_DIGEST}`.
- [ ] ECS service updates in place.
- [ ] API digest, desired count, RDS, network, secrets, and load balancer do not change.

## 5. Apply and verify

```bash
terraform -chdir="${TF_ROOT}" apply staging-storefront-rollout.tfplan

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

terraform -chdir="${TF_ROOT}" plan
```

- [ ] Desired and running counts match; pending is `0`.
- [ ] Homepage, health, API readiness, and affected product flows work.
- [ ] Staging still returns no-index controls.
- [ ] Final Terraform plan says `No changes`.

## Roll back

Set `storefront_image_digest` in
`infrastructure/terraform/staging/application/variables.tf` to the previous
known-good ECR digest. Commit the pin, create and review a new saved Terraform
plan, apply it, wait for the ECS service to stabilize, and repeat every
verification in step 5. Do not reuse the original plan and do not delete the
previous image during a release.
