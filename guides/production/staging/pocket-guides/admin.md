# Pocket guide: staging admin release

| Release record | Value |
|---|---|
| Operator / date | ______________________________ |
| Source commit | ______________________________ |
| CloudFront invalidation ID | ______________________________ |

The admin is static: local build → private S3 → CloudFront. No Docker, ECS, or
Terraform is needed for an ordinary code release. New hashed assets are
uploaded first, `index.html` is uploaded last, and old assets are retained until
CloudFront finishes invalidating cached files. Stop immediately when a command
or checklist gate fails.

## 1. Test, build, and commit

```bash
npm ci
npm run typecheck --workspace=@knitnprint/admin
VITE_STOREFRONT_URL=https://staging.knitnprint.com \
  npm run build --workspace=@knitnprint/admin

test -f apps/admin/dist/index.html
test -f apps/admin/dist/robots.txt
rg -n 'noindex|nofollow' apps/admin/dist/index.html
rg -n 'Disallow: /' apps/admin/dist/robots.txt
git status --short
```

- [ ] Build and no-index checks pass.
- [ ] Commit source changes before uploading. Do not commit `dist`.

## 2. Authenticate and preview the upload

```bash
aws sso login --profile knitnprint-administrator

aws s3 sync apps/admin/dist \
  s3://knitnprint-staging-admin-assets-739863594156/ \
  --exclude index.html \
  --cache-control no-cache \
  --dryrun \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager
```

- [ ] Review every listed upload.
- [ ] Bucket name contains `staging`.

## 3. Upload assets first and index last

```bash
aws s3 sync apps/admin/dist \
  s3://knitnprint-staging-admin-assets-739863594156/ \
  --exclude index.html \
  --cache-control no-cache \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager

aws s3 cp apps/admin/dist/index.html \
  s3://knitnprint-staging-admin-assets-739863594156/index.html \
  --content-type text/html \
  --cache-control no-cache \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager
```

## 4. Invalidate CloudFront

```bash
INVALIDATION_ID="$(
  aws cloudfront create-invalidation \
    --distribution-id E1LIGJBBJV4A65 \
    --paths '/*' \
    --profile knitnprint-administrator \
    --query 'Invalidation.Id' \
    --output text \
    --no-cli-pager
)"

aws cloudfront wait invalidation-completed \
  --distribution-id E1LIGJBBJV4A65 \
  --id "${INVALIDATION_ID}" \
  --profile knitnprint-administrator
```

## 5. Preview stale-asset removal

```bash
aws s3 sync apps/admin/dist \
  s3://knitnprint-staging-admin-assets-739863594156/ \
  --exclude index.html \
  --delete \
  --cache-control no-cache \
  --dryrun \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager
```

- [ ] Only obsolete generated assets are listed.
- [ ] If correct, remove them with the command below.

```bash
aws s3 sync apps/admin/dist \
  s3://knitnprint-staging-admin-assets-739863594156/ \
  --exclude index.html \
  --delete \
  --cache-control no-cache \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager
```

## 6. Verify

```bash
curl --fail-with-body --silent --show-error \
  https://admin.staging.knitnprint.com/ >/dev/null
curl --fail-with-body --silent --show-error \
  https://admin.staging.knitnprint.com/robots.txt
curl --silent --show-error --head \
  https://admin.staging.knitnprint.com/ \
  | rg -i 'http/|x-robots-tag|strict-transport-security|x-content-type-options|x-frame-options'

curl --silent --show-error \
  --output /dev/null \
  --write-out '%{http_code}\n' \
  https://admin.staging.knitnprint.com/api/admin/auth/me
```

- [ ] Unauthenticated API check returns `401`.
- [ ] No-index and security headers are present.
- [ ] Sign in and smoke-test the changed admin flow.

## Roll back

Build the previous known-good commit in a separate clean worktree. Upload it
using the same asset-first, `index.html`-last sequence, create a new CloudFront
invalidation, wait for it to complete, and repeat the verification in step 6.
Never make the S3 bucket public.
