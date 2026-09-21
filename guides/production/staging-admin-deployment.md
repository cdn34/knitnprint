# Deploy admin changes to staging

The admin application is static. It is built locally, uploaded to the private
`knitnprint-staging-admin-assets-739863594156` bucket, and served through
CloudFront distribution `E1LIGJBBJV4A65`. It does not use a Docker image or ECS
service.

## 1. Test and build

```bash
npm ci
npm run typecheck --workspace=@knitnprint/admin
VITE_STOREFRONT_URL=https://staging.knitnprint.com \
  npm run build --workspace=@knitnprint/admin
```

Confirm the build exists and still blocks indexing:

```bash
test -f apps/admin/dist/index.html
test -f apps/admin/dist/robots.txt
rg -n 'noindex|nofollow' apps/admin/dist/index.html
rg -n 'Disallow: /' apps/admin/dist/robots.txt
```

Commit the source changes before uploading. The generated `dist` directory is a
deployment artifact, not the source of truth.

## 2. Authenticate and review the upload

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

Review every upload. Do not use a production bucket.

## 3. Upload without breaking the current page

Upload new assets first and the entry page last. Keep old hashed assets until
the CloudFront invalidation finishes.

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

Invalidate CloudFront and wait for completion:

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

After the invalidation completes, review and remove stale objects:

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

If the dry run lists only obsolete build artifacts, repeat it without
`--dryrun`.

## 4. Verify

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

The unauthenticated API check should return `401`, proving CloudFront routes
`/api/*` to the staging API. Then sign in and smoke-test the changed admin flow.

Terraform is not required for ordinary admin code releases. Use Terraform only
when changing the bucket policy, CloudFront distribution, certificate, headers,
or origins.

## Roll back

Build the previous known-good commit in a separate worktree, upload it using the
same asset-first/index-last sequence, and invalidate CloudFront again. Do not
roll back by making the S3 bucket public.
