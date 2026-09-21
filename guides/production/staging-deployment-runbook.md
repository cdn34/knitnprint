# Staging deployment execution runbook

Last updated: 2026-09-15

This runbook records the completed KnitNPrint staging-deployment work in chronological order so the preparation can be audited or repeated. Application preparation, Terraform installation, IAM Identity Center setup, the staging-relevant AWS audit, Terraform bootstrap, and DNS/SES prerequisites are complete. The application network foundation is drafted locally but has not been created in AWS.

Related documents:

- [Staging deployment plan](./staging-deployment-plan.md) — target architecture and remaining work.
- [Current staging deployment handoff](./staging-deployment-handoff.md) — exact resume checkpoint, verified AWS identity state, partial audit findings, and next commands.
- [AWS identities and staging credentials](./aws-identity-and-staging-credentials.md) — human access, temporary credentials, deployment permission sets, and workload roles.
- [Terraform installation on WSL Ubuntu](./terraform-wsl-installation.md) — detailed installation and repair notes.
- [Production launch infrastructure](./launch-infrastructure.md) — broader production requirements and cost assumptions.
- [cURL study notes](../tools/curl-study-notes.md) — HTTP diagnostics used throughout deployment work.
- [Scheduled workers](./staging-scheduled-workers.md) — add and verify periodic one-shot Fargate jobs.
- [Storefront releases](./staging-storefront-deployment.md) — build, publish, roll out, and verify the storefront image.
- [Admin releases](./staging-admin-deployment.md) — publish the static admin build through private S3 and CloudFront.
- [Backend releases](./staging-backend-deployment.md) — publish and roll out the Rust API image.
- [Database operations](./staging-database-operations.md) — migrations, operational checks, bounded queries, and customer-data cleanup.

## Current checkpoint

| Phase | Status | Evidence |
| --- | --- | --- |
| Deployment architecture and staging plan | Complete | Commit `47c0120` |
| `APP_ENV=staging` and KnitNPrint naming correction | Complete | Commit `ed3a125` |
| Staging email-recipient allowlist | Complete | Commit `4c3ce83` |
| Deployable storefront Node runtime | Complete | Commit `9dbcd49` |
| Admin static deployment/indexing policy | Complete | Commit `89ed17b` |
| Backend operational binaries in runtime image | Complete | Commit `a3bd602` |
| Restricted operational database behavior | Complete | Commit `8b669d7` |
| Trusted-proxy client-IP hardening | Complete | Commit `547adcf` |
| Nitro storefront API-flow correction | Complete | Commit `f7aa77b` |
| Full local browser and container acceptance | Complete | 24/24 Playwright tests and both image smoke tests passed |
| Terraform CLI installation | Complete | Terraform `1.16.0`, Debian package `1.16.0-1`, APT hold active |
| AWS login and read-only account audit | Complete | Identity, IAM, SES, DNS, budget, storage, and active-resource inventory verified |
| Shared MinIO/AWS S3 object-storage strategy | Complete | Five focused tests, full Rust workspace, and strict Clippy pass |
| Terraform bootstrap configuration | Complete | Reviewed plan applied: 10 added, 0 changed, 0 destroyed |
| Terraform remote state | Complete | Versioned, encrypted S3 object at `staging/bootstrap/terraform.tfstate`; post-migration plan is empty |
| Human Terraform access | Complete | All roots use `knitnprint-administrator`; redundant staging permission set was removed through Terraform |
| DNS/SES prerequisite configuration | Complete | Seven records are authoritative; both ACM certificates, SES identity, DKIM, and MAIL FROM succeeded |
| Application network foundation | Complete | 17-resource plan applied; live routing/security checks and no-change plan passed |
| Application storage and ECR | Complete | 19-resource plan applied; live protection/retention checks and no-change plan passed |
| Application database | Complete | 3-resource plan applied; live RDS/TLS/secret checks and no-change plan passed |
| Application load balancer | Complete | 6-resource plan applied; live listener/routing checks and no-change plan passed |
| ECS runtime foundation | Complete | 12-resource plan applied; zero tasks/services, scoped IAM, empty secrets, and no-change plan verified |
| Immutable API and storefront images | Complete | Hardened commit `e82660e`; both pushed to private ECR and passed scan-on-push with zero findings |
| ECS service and database operation definitions | Complete | 8-resource plan applied; stopped service/task definitions, scoped bootstrap role, and no-change plan verified |
| Database application credentials | Complete | Separate generated migration/runtime secrets have current versions; values were not displayed or read during verification |
| Database role bootstrap | Complete | Corrected revision 2 exited zero; role capabilities verified through a read-only ECS check |
| Database migrations | Complete | Migration task exited zero; 24 successful, 0 failed, and 50 public tables verified |
| Stripe test secret | Complete | Secure script populated an AWSCURRENT version; values were not read during verification |
| Running ECS application | Complete | One healthy task; completed rollout; API and storefront targets healthy; post-apply plan has no drift |
| Staging indexing protection | Complete | Revision 2 is healthy; live header, SSR meta, robots.txt, API health, logs, targets, and no-drift checks pass |
| Storefront application DNS | Complete | Namecheap CNAME targets the staging ALB; public-host HTTPS, API, and noindex checks pass |
| Admin S3/CloudFront application | Draft validated | Admin link/build corrected; CloudFront/OAC, private bucket grant, API behavior, and outputs validate; plan/apply/upload/DNS remain |
| Initial owner and store configuration | Pending | Create owner safely, verify admin login, then configure store data |
| End-to-end staging acceptance | Pending | Verify auth, catalog, signed media, checkout/webhooks, email, and noindex controls |
| Operational automation | Deferred | Scheduled jobs, lean alarms, SES production access, and GitHub OIDC releases follow first staging review |

The work was performed on branch `carlosnogueira/staging-deployment`, based on `master` commit `b247524`. The current application checkpoint is commit `e82660e`.

Confirm that history with:

```bash
git branch --show-current
git merge-base master HEAD
git rev-parse HEAD
git log --reverse --format='%h %s' b247524..HEAD
```

Expected commit sequence:

```text
47c0120 docs: deployment guides
ed3a125 feat: add staging mode and correct KnitNPrint naming
4c3ce83 feat: restrict staging email recipients
9dbcd49 feat: add deployable storefront runtime
89ed17b feat: define admin deployment policy
a3bd602 feat: package backend operational binaries
8b669d7 fix: restrict operational database privileges
547adcf feat: harden trusted proxy client identity
f7aa77b fix: preserve storefront API flows with Nitro
320fb2f feat: separate deployed object storage and staging terraform
e82660e fix: harden deployment runtime images
```

## Safety and replay rules

- Run repository commands from the repository root.
- Replace example domains, addresses, and credentials before a real deployment.
- Values such as `knitnprint:knitnprint` in this guide are local-test credentials only.
- Never commit AWS credentials, Stripe keys, database passwords, OAuth tokens, or Terraform state.
- Stop when a command fails. Preserve its complete output before changing anything else.
- Do not use the AWS root user to bypass an IAM error.
- Do not run `terraform apply` until a saved plan has been reviewed.
- Do not run `docker compose down --volumes` against existing local data merely to resolve the KnitNPrint rename. Existing pre-rename volumes can contain the old database roles, database name, MinIO credentials, or bucket name.
- Commands under “Historical implementation command” explain how a dependency or artifact was originally introduced. A normal replay from the committed branch should use `npm ci`, not repeat an unpinned package installation.

Check the worktree before every phase:

```bash
git status --short
```

Untracked or modified files that are not part of the current phase must not be staged, overwritten, or deleted.

## 1. Record the architecture and deployment decisions

The first completed slice documented:

- one existing AWS account with separate staging and production resources;
- `eu-west-1` as the main AWS Region;
- external DNS management;
- Terraform-managed infrastructure;
- ECS Fargate for the Rust API, storefront, workers, and ClamAV;
- private S3 for media and the admin build;
- RDS PostgreSQL;
- SES transactional email;
- Stripe test mode in staging;
- on-demand staging start/stop behavior;
- CloudFront's ACM certificate in `us-east-1`;
- staging and production cost expectations.

The documentation commit can be inspected with:

```bash
git show --stat 47c0120
git show 47c0120 -- guides/production/staging-deployment-plan.md
```

No cloud resources were created during this phase.

## 2. Add staging behavior and correct the KnitNPrint identity

### Application changes

Commit `ed3a125` introduced an explicit `Staging` environment and corrected the previous `KnitPrint`/`knitprint` spelling throughout application commands, packages, images, containers, databases, buckets, generated API artifacts, documentation, and tests.

Staging now follows deployed-environment safeguards:

- `DATABASE_URL` is mandatory.
- Web origins must be explicit HTTPS origins.
- Secure cookies are enabled.
- Structured production-style logging is used.
- Manual payments are disabled.
- S3, malware scanning, and SES configuration are required.
- Stripe must use a test-mode `sk_test_` key.
- A live Stripe key is rejected.
- Migrations do not run during API startup.

The identity correction included migration `0024_correct_knitnprint_identity.sql`, which intentionally retains the old strings only as values to be migrated. Therefore, this audit should report legacy spellings only in the historical migration files:

```bash
rg -n -i 'knitprint' \
  --glob '!target/**' \
  --glob '!node_modules/**' \
  --glob '!.git/**' \
  .
```

The expected remaining matches are the old values in migrations `0020` and `0024`; active commands and feature names should use `knitnprint`.

Audit all environment branches:

```bash
rg -n 'APP_ENV|Environment::(Development|Test|Staging|Production)' \
  backend/src backend/.env.example
```

### Dependency and generated-artifact synchronization

During the implementation, workspace metadata and generated API files were refreshed. On a clean replay of the committed branch, use:

```bash
npm ci
npm run api:generate
```

### Validation commands

```bash
cargo fmt --all
npm run typecheck
npm run build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Inspect the completed slice:

```bash
git show --stat ed3a125
```

## 3. Restrict staging email recipients

Commit `4c3ce83` added `EMAIL_RECIPIENT_ALLOWLIST`.

The completed behavior is:

- staging refuses startup without a valid, non-empty allowlist;
- addresses are trimmed, normalized to lowercase, deduplicated, and matched exactly;
- verification, password-reset, order, and fulfilment messages are checked centrally;
- a blocked recipient is rejected before SES is called;
- production remains unrestricted unless an allowlist is explicitly configured.

Example staging configuration:

```text
APP_ENV=staging
EMAIL_DELIVERY=ses
EMAIL_FROM=no-reply@staging.knitnprint.com
EMAIL_RECIPIENT_ALLOWLIST=owner@example.com,tester@example.com
AWS_REGION=eu-west-1
SES_CONFIGURATION_SET=knitnprint-staging-transactional
```

Do not copy example recipients unchanged. Use only explicitly approved test addresses.

Run the focused and full validations:

```bash
cargo fmt --all
cargo test -p knitnprint-api email::tests
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

The focused email suite reported eight passing tests during this phase.

Inspect the commit:

```bash
git show --stat 4c3ce83
```

## 4. Create a deployable storefront runtime

Commit `9dbcd49` added:

- a TanStack Start/Nitro production runtime;
- a production `start` script;
- a `/health` route;
- a pinned, multi-stage, non-root storefront Docker image;
- server-only `API_BASE_URL` handling for SSR requests;
- browser-relative `/api/...` requests;
- supporting generated route and package-lock updates.

### Historical implementation command

Nitro was initially added with:

```bash
npm install --workspace=@knitnprint/storefront nitro@npm:nitro-nightly@latest
```

Do not replay `@latest`. The committed dependency is pinned by `package.json` and `package-lock.json`. Reproduce it with:

```bash
npm ci
```

### Build and typecheck

```bash
npm run typecheck --workspace=@knitnprint/storefront
npm run build --workspace=@knitnprint/storefront
```

### Start the production runtime locally

In terminal one:

```bash
PORT=3010 \
HOST=127.0.0.1 \
API_BASE_URL=http://127.0.0.1:8080 \
npm run start --workspace=@knitnprint/storefront
```

Expected startup line:

```text
Listening on: http://127.0.0.1:3010/
```

In terminal two:

```bash
curl --fail --show-error --include http://127.0.0.1:3010/health
curl --fail --show-error --include http://127.0.0.1:3010/
```

Stop the server in terminal one with `Ctrl-C` after verification.

Inspect the commit:

```bash
git show --stat 9dbcd49
```

## 5. Define the admin static-deployment policy

Commit `89ed17b` prepared the admin Vite SPA for a private S3 origin behind CloudFront.

The completed policy requires:

- a private S3 bucket;
- CloudFront Origin Access Control;
- `index.html` fallback for client-side routes;
- a CloudFront response-headers policy adding `X-Robots-Tag: noindex, nofollow`;
- a matching robots meta tag;
- a `robots.txt` that disallows all crawlers.

Build and inspect the admin output:

```bash
npm run typecheck --workspace=@knitnprint/admin
npm run build --workspace=@knitnprint/admin
sed -n '1,80p' apps/admin/dist/index.html
sed -n '1,20p' apps/admin/dist/robots.txt
```

The static files are prepared locally. The private S3 bucket, CloudFront distribution, response-headers policy, and SPA error mapping have not yet been created.

Inspect the commit:

```bash
git show --stat 89ed17b
```

## 6. Package backend operational binaries

Commit `a3bd602` changed the backend image so ECS can run the same artifact as the API or as a one-off/scheduled job.

The runtime image contains:

```text
knitnprint-api
migrate
create_owner
deliver_notifications
cleanup_sessions
cleanup_customers
cleanup_carts
cleanup_media
cleanup_payments
check_operations
```

The development-only `seed` binary is intentionally absent, and the container runs as UID/GID `10001`.

Build the image:

```bash
docker build --file backend/Dockerfile --tag knitnprint-api:test .
```

Inspect its default process and user, then verify every operational binary:

```bash
docker image inspect knitnprint-api:test \
  --format '{{json .Config.Cmd}} {{json .Config.Entrypoint}} {{.Config.User}}'

docker run --rm --entrypoint /bin/sh knitnprint-api:test -c '
  set -eu
  test "$(id -u)" = 10001
  for binary in \
    knitnprint-api migrate create_owner deliver_notifications \
    cleanup_sessions cleanup_customers cleanup_carts cleanup_media \
    cleanup_payments check_operations
  do
    test -x "/usr/local/bin/$binary"
  done
  test ! -e /usr/local/bin/seed
  echo operational-binaries-ok
'
```

Expected final line:

```text
operational-binaries-ok
```

Smoke-test the API image:

```bash
docker run --detach --rm \
  --name knitnprint-api-smoke \
  --publish 127.0.0.1:8082:8080 \
  knitnprint-api:test

curl --fail --show-error http://127.0.0.1:8082/api/health
docker logs knitnprint-api-smoke
docker stop knitnprint-api-smoke
```

Inspect the commit:

```bash
git show --stat a3bd602
```

## 7. Restrict operational database privileges

Commit `8b669d7` removed implicit migration execution from cleanup and owner commands.

The operational rule is now:

1. Run `migrate` explicitly with the migration/schema-owner credential.
2. Run the API and jobs with their narrower runtime/job credentials.
3. Never make a cleanup or owner task silently elevate itself by applying migrations.
4. Require explicit staging/production S3 configuration for media cleanup and use the ECS task role.

Audit migration calls in command binaries:

```bash
rg -n 'sqlx::migrate' backend/src/bin --glob '*.rs'
```

Only the dedicated migration binary or explicitly intended development paths should apply migrations.

Validate all binaries and the workspace:

```bash
cargo fmt --all --check
cargo test --workspace --bins
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Inspect the commit:

```bash
git show --stat 8b669d7
```

## 8. Harden trusted-proxy client identity

Commit `547adcf` added `TRUSTED_PROXY_HOPS` and changed forwarded-address handling so a caller-supplied leftmost `X-Forwarded-For` entry cannot spoof rate-limit identity behind an AWS ALB.

For the planned single ALB hop:

```text
TRUST_PROXY_HEADERS=true
TRUSTED_PROXY_HOPS=1
```

Never enable trusted proxy headers while clients can connect directly to the API. The staging security group must permit API ingress only from the ALB security group.

Run focused tests:

```bash
cargo test -p knitnprint-api login_rate_limit::tests
cargo test -p knitnprint-api config::tests
```

Run full validation:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Inspect the commit:

```bash
git show --stat 547adcf
```

## 9. Preserve storefront API flows with Nitro

Initial Nitro output exposed integration differences that affected existing browser API behavior. Commit `f7aa77b` corrected the Vite/Nitro configuration, container build, styles, and end-to-end coverage.

Build and typecheck the final storefront:

```bash
npm run typecheck --workspace=@knitnprint/storefront
npm run build --workspace=@knitnprint/storefront
```

Verify that the public browser bundle does not contain the server-only internal API address:

```bash
if rg -n '127\.0\.0\.1:8080|API_BASE_URL' apps/storefront/.output/public; then
  echo 'unexpected internal API address in browser bundle' >&2
  exit 1
else
  echo 'browser-bundle-api-address-ok'
fi
```

Expected result:

```text
browser-bundle-api-address-ok
```

Inspect the commit:

```bash
git show --stat f7aa77b
```

## 10. Run the full local acceptance sequence

The final local acceptance deliberately used an isolated PostgreSQL container on port `55432`. This avoided modifying or deleting existing Compose volumes whose pre-rename identities might no longer match `compose.yaml`.

### Start isolated PostgreSQL

```bash
docker run --detach --rm \
  --name knitnprint-e2e-postgres \
  --publish 127.0.0.1:55432:5432 \
  --env POSTGRES_USER=knitnprint \
  --env POSTGRES_PASSWORD=knitnprint \
  --env POSTGRES_DB=knitnprint \
  postgres:17-alpine
```

Wait until it is ready:

```bash
docker exec knitnprint-e2e-postgres \
  pg_isready -U knitnprint -d knitnprint
```

Expected result includes:

```text
accepting connections
```

### Apply migrations

```bash
DATABASE_URL=postgres://knitnprint:knitnprint@127.0.0.1:55432/knitnprint \
cargo run --quiet -p knitnprint-api --bin migrate
```

### Run all storefront browser tests

```bash
DATABASE_URL=postgres://knitnprint:knitnprint@127.0.0.1:55432/knitnprint \
npx playwright test --workers=1
```

Playwright starts the Rust API and storefront development server from `playwright.config.ts`. The completed run reported:

```text
24 passed
```

If the servers do not shut down cleanly, inspect them without killing unrelated processes:

```bash
ss -ltnp | rg ':3000|:8080|:55432' || true
ps -ef \
  | rg 'target/debug/knitnprint-api|vite|playwright' \
  | rg -v rg \
  || true
```

### Build and smoke-test the final storefront image

```bash
docker build \
  --file apps/storefront/Dockerfile \
  --tag knitnprint-storefront:test \
  .

docker image inspect knitnprint-storefront:test \
  --format '{{json .Config.Cmd}} {{json .Config.Entrypoint}} {{.Config.User}}'
```

The configured user should be `knitnprint` and the command should run `.output/server/index.mjs`.

```bash
docker run --detach --rm \
  --name knitnprint-storefront-smoke \
  --publish 127.0.0.1:3011:3000 \
  --add-host host.docker.internal:host-gateway \
  --env API_BASE_URL=http://host.docker.internal:8080 \
  knitnprint-storefront:test

curl --fail --show-error http://127.0.0.1:3011/health
docker logs knitnprint-storefront-smoke
docker stop knitnprint-storefront-smoke
```

The health request passed and the image ran as its non-root user.

### Stop and remove the temporary database container

```bash
docker stop knitnprint-e2e-postgres
```

Because the container was started with `--rm`, stopping it removed it. No persistent database volume was created or deleted.

### Final static validations

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run typecheck
npm run build
git diff --check
git status --short
```

All application-side checks were green at commit `f7aa77b`.

## 11. Install and pin Terraform on WSL Ubuntu

The workstation is Ubuntu 24.04 `noble` on AMD64. Terraform was installed from HashiCorp's signed APT repository and pinned at `1.16.0`.

### Identify the platform

```bash
grep -E '^(ID|VERSION_ID|VERSION_CODENAME|UBUNTU_CODENAME)=' /etc/os-release
dpkg --print-architecture
```

Observed values:

```text
VERSION_ID="24.04"
VERSION_CODENAME=noble
ID=ubuntu
UBUNTU_CODENAME=noble
amd64
```

### Install prerequisites and HashiCorp's signing key

```bash
sudo apt-get update
sudo apt-get install -y gnupg software-properties-common wget

wget -O- https://apt.releases.hashicorp.com/gpg \
  | gpg --dearmor \
  | sudo tee /usr/share/keyrings/hashicorp-archive-keyring.gpg >/dev/null

gpg --no-default-keyring \
  --keyring /usr/share/keyrings/hashicorp-archive-keyring.gpg \
  --fingerprint
```

The required fingerprint is:

```text
798A EC65 4E5C 1542 8C8E 42EE AA16 FCBC A621 E701
```

Stop if the fingerprint differs.

### Write the APT source as one physical line

The initial source entry was malformed because the bracketed options, URI, and `main` component were written on separate physical lines. The corrected reusable command is:

```bash
source /etc/os-release
terraform_architecture="$(dpkg --print-architecture)"
terraform_ubuntu_codename="${UBUNTU_CODENAME:-$VERSION_CODENAME}"

printf 'deb [arch=%s signed-by=%s] %s %s main\n' \
  "$terraform_architecture" \
  '/usr/share/keyrings/hashicorp-archive-keyring.gpg' \
  'https://apt.releases.hashicorp.com' \
  "$terraform_ubuntu_codename" \
  | sudo tee /etc/apt/sources.list.d/hashicorp.list
```

Verify exactly one line:

```bash
wc -l /etc/apt/sources.list.d/hashicorp.list
sed -n '1p' /etc/apt/sources.list.d/hashicorp.list
```

Expected source on this workstation:

```text
deb [arch=amd64 signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com noble main
```

### Commands used to diagnose and repair the malformed source

These commands distinguished real newline bytes from terminal wrapping:

```bash
sudo sed -n '1,5p' /etc/apt/sources.list.d/hashicorp.list
od -An -tx1c /etc/apt/sources.list.d/hashicorp.list
sed -n 'l' /etc/apt/sources.list.d/hashicorp.list
ls -l /etc/apt/sources.list.d/hashicorp.list
```

The repair used a known-good temporary file:

```bash
printf '%s%s%s\n' \
  'deb [arch=amd64 signed-by=' \
  '/usr/share/keyrings/hashicorp-archive-keyring.gpg] ' \
  'https://apt.releases.hashicorp.com noble main' \
  > /tmp/hashicorp.list

wc -l /tmp/hashicorp.list
sed -n '1p' /tmp/hashicorp.list
sudo cp /tmp/hashicorp.list /etc/apt/sources.list.d/hashicorp.list
```

After APT accepted the source, the temporary file was removed:

```bash
rm /tmp/hashicorp.list
```

### Refresh APT, select the exact version, and install

```bash
sudo apt-get update
apt-cache madison terraform | head -10
sudo apt-get install -y terraform=1.16.0-1
sudo apt-mark hold terraform
```

### Verify the installation

```bash
command -v terraform
terraform version
dpkg-query -W -f='${Package} ${Version} ${Architecture}\n' terraform
apt-mark showhold | grep -x terraform
apt-cache policy terraform | sed -n '1,8p'
```

Verified result:

```text
/usr/bin/terraform
Terraform v1.16.0
on linux_amd64
terraform 1.16.0-1 amd64
terraform
```

The installed and candidate APT package versions were both `1.16.0-1`, and `terraform` appeared in the APT hold list.

## 12. Handoff at the current stage

The local application is deployment-ready for the next infrastructure slice, Terraform is installed, IAM Identity Center administrator access is verified, and the staging-relevant read-only account audit is complete. See the [current staging deployment handoff](./staging-deployment-handoff.md) for the full evidence, findings, and exact stopping point.

The staging bootstrap is now prepared and validated at
`infrastructure/terraform/staging/bootstrap`. It will create the dedicated
`knitnprint-staging-terraform-state-<account-id>` bucket, the staging budget,
and the initial `KnitNPrintStagingDeployer` assignment. A successful plan found
10 additions and no changes or deletions, but it was discarded before apply
after review found that its first policy draft could modify non-staging
resources through broad service wildcards. The policy now grants only safe
identity discovery and exact staging-state access. The replacement saved plan
has passed human and JSON review with exactly 10 creates, one data-source read,
and no updates, replacements, or deletions. Its SHA-256 is
`b31e4f820c41cdcbdb937e2a1052e451b576a0dd6c3f27cbb7e2764d86c64c0d`.
That exact plan was applied successfully on 2026-09-02: 10 resources were
added, none changed, and none destroyed. Read-only AWS verification confirmed
the bucket protections, three healthy budget notifications, hardened inline
policy, group assignment, and generated Identity Center role. The local state
was then migrated into the S3 backend. Terraform can read all 10 managed
resources from it, no active lock object remains, and a refresh plan reports no
changes.

The environment layout and naming contract are recorded in
`infrastructure/terraform/README.md`. Staging and production use independent
Terraform roots, state buckets, deployers, workload resources, media buckets,
and admin-assets buckets. Staging resources use `knitnprint-staging-*`; future
production resources use `knitnprint-production-*`.

The DNS root was initialized successfully against the remote S3 backend and its
first plan proposed nine additions with no changes or deletions. Do not apply
that plan. Review found two hardening changes to make before creating resources:

- keep SES configuration-set reputation metrics disabled to avoid separately
  billed CloudWatch metrics while SNS records bounce, complaint, reject, and
  delivery-delay events;
- set both ACM public certificates to `export = "DISABLED"` and require
  `acm:Export = DISABLED` in the staging deployer's request permission.

The obsolete plan was retained only as the ignored file
`staging-dns.pre-cost-hardening.stale-do-not-apply.tfplan`.

The DNS backend was subsequently reconfigured for
`knitnprint-administrator`, and a live plan returned `No changes`. The
application root was initialized successfully against
`staging/application/terraform.tfstate`.

The first application plan was then saved and reviewed. It proposes exactly 17
creates, no updates or deletions, and no sensitive values. The plan contains
only the VPC, Internet Gateway, four subnets, route tables and associations, and
three security groups; it contains no compute, load balancer, database, NAT
Gateway, storage, or allocated public IP resource. Its SHA-256 is
`046105c919ec24b0e2ff8177c5096364047829260775551c59f966e1bb1c1521`.

The exact saved plan applied successfully with 17 additions, no changes, and no
deletions. Live AWS checks confirmed the `10.40.0.0/20` VPC, two public and two
isolated database subnets across `eu-west-1a` and `eu-west-1b`, correct route
associations, and the three reviewed security-group boundaries. A post-apply
Terraform plan returned `No changes`. The encrypted remote application state
object has version ID `zBxR8uHTCLl4rI7KRlFt2pdnBUirYbQ4`.

The storage and registry configuration was then added to the same application
root. Exact-name AWS collision checks returned no matching buckets or ECR
repositories. Its saved plan contains 19 creates, 17 network no-ops, two
deferred policy-document reads, no updates or deletions, and no sensitive
values. It passed review for private access, ownership enforcement, encryption,
versioning, bounded old-version/image retention, TLS-only S3 access, restricted
media CORS, immutable ECR tags, and image scanning. The plan SHA-256 is
`aa382624e5dcfc9bcf2bf6a8007a36600b3c5bcbe2ad085358e4dd4508d77199`.

The exact plan applied with 19 additions, no changes, and no deletions. Direct
AWS checks confirmed every reviewed S3 and ECR control, and the post-apply
Terraform plan returned `No changes`. The encrypted remote application state
advanced to version `IC9WiELHFBVXXiU5Zoy_Besk1pOxsxlA`.

The database slice passed provider validation, current Ireland orderability
checks for PostgreSQL 17.9 on `db.t4g.micro` with gp3, and exact-name collision
checks. Its saved plan contains three creates, 36 no-ops, no updates or
deletions, and no password value. Review confirmed private networking,
Single-AZ placement, 20 GB encrypted storage, forced TLS, seven-day backups,
RDS-managed master credentials, deletion protection, required final snapshot,
and disabled optional monitoring. The plan SHA-256 is
`4cae055dbb10ea7d71d04b183c7535e15b85cf7d79970b831eb76fffea75c5a4`.

That exact database plan applied with three additions, no changes, and no
deletions. The RDS instance became `available` at
`knitnprint-staging-postgres.cdugyku62w1w.eu-west-1.rds.amazonaws.com:5432`.
Read-only AWS checks confirmed PostgreSQL 17.9, `db.t4g.micro`, 20 GB gp3,
private access, encryption, Single-AZ placement, seven-day backups, deletion
protection, and `rds.force_ssl = 1`. The RDS-owned Secrets Manager secret has
rotation enabled; its secret value was not retrieved. AWS reports the static
TLS parameter with `pending-reboot`, so the Terraform declaration was aligned
with that normalized API value. Provider validation and a full post-apply plan
then passed with no changes. The encrypted remote application state advanced
to version `LNibNicwWFxl0rzHxz_GnYcJLMBKj7qi`.

The load-balancer plan contained six creates, all existing resources as no-ops,
and no updates or deletions. Its reviewed SHA-256 was
`d3d149f9a213d94a98221b9253c596aa232a5a0967076ccfbac0f2ad0f439011`.
The exact plan applied successfully. Direct AWS inspection confirmed the
internet-facing ALB is active in both public subnets, port 80 redirects to
HTTPS, port 443 uses the issued `staging.knitnprint.com` certificate and the
TLS 1.2/1.3 policy, `/api` and `/api/*` route to the API target group, and the
default action routes to the storefront target group. Both target groups are
intentionally empty until ECS services are deployed. The post-apply Terraform
plan reported no changes. The encrypted remote application state advanced to
version `2KXYlGC4U3PYXTIECqq3EmZCQVyqJAP8`.

The regenerated runtime-foundation plan contained exactly 12 creates and no
updates or deletions. Its reviewed SHA-256 was
`8997d979142896712c76c223eb99b4ea38bcfb376b5763bfd329e4d8203244e9`.
The exact plan applied successfully. It created the `knitnprint-staging` ECS
cluster, three 14-day log groups, the ECS execution and API task roles, and
empty secret containers for runtime database, migration database, and Stripe
test credentials. The general execution role cannot read the RDS master
credential; its secret permission resolves only to the three application
secrets. The API role is limited to staging media objects and the staging SES
sender/configuration set. Live checks confirmed Container Insights is disabled,
there are zero ECS services or tasks, the log groups contain zero bytes, and
all three secret containers have zero versions. No secret value was retrieved.
The post-apply Terraform plan reported no changes. The encrypted remote
application state advanced to version `KduRXdKi5J3esAHixappXc_2OsETXxK8`.

### Immutable image publication and stopped ECS definitions

The deployment images were built from commit
`e82660e316202d420964b5b38d33aeeb26722407` and tagged with that complete Git
SHA. The API runtime uses a pinned non-root distroless base. The storefront
runtime upgrades Alpine packages during its pinned image build and runs as the
non-root `knitnprint` user. Local container health checks passed before upload.

The images were uploaded to the private staging ECR repositories. The tagged
OCI indexes and their deployable Linux manifest digests are:

| Service | Tagged OCI index | Linux image manifest |
| --- | --- | --- |
| API | `sha256:3bfdd65ea94b55306745e061cf043173894dfd712d664ff54062f62f3536f224` | `sha256:2e872ca6d5aa9addfd4a13a993172162c4b702b240ab6b40952bfc149cd26e2c` |
| Storefront | `sha256:507addeef2118fa26fd1a7c547275348fd577ddcb82ad2721f4123cb68405029` | `sha256:71ec5a605378d5a0ffa807bb5f56b1e3099c0ace054e35257fb54ebb8fd23c84` |

ECR basic scan-on-push completed successfully for both Linux manifests on
2026-09-08. Both reports contained an empty severity-count map: zero critical,
high, medium, low, or informational findings. The previous images remain in
ECR for rollback and must not be deleted without a separate explicit review.

The next Terraform slice defines one lean Fargate service containing the API,
storefront, and a pinned official ClamAV sidecar. It also defines stopped,
one-off task definitions for database role bootstrap and SQLx migrations. The
normal service execution role still cannot read the RDS master credential; a
separate bootstrap execution role can read it only for the bootstrap task. The
service is deliberately created with desired count zero, so applying this slice
does not start Fargate compute before secret population and migrations.

Machine-readable review of `staging-ecs-definitions.tfplan` found exactly eight
creates and 57 no-ops, with no updates, deletions, replacements, or sensitive
variables. The combined task is fixed at 1 vCPU and 5 GB RAM to leave room for
ClamAV's signature database. The reviewed plan SHA-256 is
`d0004bbfd70e8cd2ec45e9a3d73b27e90038dd183182c58b014c0e6d8891a8ad`.

That exact plan applied successfully: eight resources were added, none changed,
and none destroyed. Live ECS inspection confirmed the service is active with
zero desired, running, or pending tasks; both ALB target groups are attached;
and all three task definitions use the reviewed immutable image digests. The
bootstrap log group has 14-day retention and zero stored bytes. A post-apply
Terraform plan reported no changes. The database application-secret containers
still had zero versions, and no secret value was read. The encrypted remote
application state advanced to version `WynP39LwVqv7kxdDqCFS0ODwUkWx0I7I`.

On 2026-09-14, `scripts/staging/populate-database-secrets.sh` generated separate
64-character hexadecimal passwords and stored JSON records for
`knitnprint_migration` and `knitnprint_runtime` directly in Secrets Manager.
The script requires no input by design. Read-only version inspection confirmed
one `AWSCURRENT` version on each secret, created at 07:53 local time. Neither
secret value was printed, retrieved, or placed in Terraform state.

The first bootstrap task, ID `846bb389c165417a88f79a621dfc53a9`, stopped
with exit code 3 before grants were applied. RDS administrators are not true
PostgreSQL superusers, and PostgreSQL rejected the script's redundant explicit
`NOSUPERUSER` alteration. The task definition was corrected to rely on the safe
default attributes of newly created roles and update only login passwords.
The saved correction plan changes only the bootstrap task definition: revision
1 is deregistered and a corrected revision is registered. The ECS service is a
no-op at desired count zero. The plan has no sensitive variables and SHA-256
`aae289d08696e79c38a978cabb69c7de181c3e5eb43a3824446721eb21e72e08`.

The correction plan applied and registered active bootstrap task-definition
revision 2. Task `bde9ba3ebe444f24bc48c9b5536d3433` then completed with
exit code zero. Its log showed the complete role, revoke, grant, membership,
and default-privilege sequence. A separate read-only ECS query confirmed that
the migration and runtime roles can log in, the reporting role cannot, and all
three roles are non-superuser and cannot create databases, create roles,
replicate, or bypass row-level security. Terraform reports no drift. The
encrypted application state advanced to version
`z2R4dZok7q8ekCYECxAuIsyLpqTgzeEP`.

Migration task `1f70eb24b19742a7adaa48e582e9bda7` used the restricted
`knitnprint_migration` credential and exited zero with `database migrations
applied`. A separate read-only query from inside the VPC found 24 successful
SQLx migration records, zero failed records, and 50 public base tables. No
database credential was displayed or retrieved during either check.

On resuming on 2026-09-15, routine AWS verification was blocked by an expired
administrator SSO token. Renew it with
`aws sso login --profile knitnprint-administrator`; Codex checks Stripe secret
version metadata next, without reading its value. If it is still empty, run
`./scripts/staging/populate-stripe-secret.sh` from an interactive terminal once
the Stripe test API key and webhook signing secret are available. Keep both
credentials out of chat, shell history, and Terraform. The service remains at
desired count zero until this prerequisite is complete.

The renewed session subsequently confirmed that the Stripe secret has no
versions. The ECS service is active on application revision 1 with desired,
running, and pending counts all zero.

On 2026-09-17 the secure population script added an `AWSCURRENT` Stripe secret
version. Read-only checks inspected metadata only. The storefront and admin ACM
certificates are issued, while the SES identity, DKIM, and custom MAIL FROM are
all verified. The public `staging.knitnprint.com` application record is not yet
published. With database migrations and Stripe prerequisites complete, the
Terraform default application desired count is now one. Review its saved plan,
apply it, verify ECS/ALB health, and only then publish/test application DNS.

The reviewed `staging-service-start.tfplan` changes only the existing ECS
service desired count from zero to one. It adds and destroys nothing, and has
SHA-256
`92bab196965fbbc9176695b0205084fcc864e0baf3ab88324e01199d49f60962`.

### Configure Stripe test credentials

1. Sign into Stripe and switch to a sandbox/test environment, not live mode.
2. Open API keys and reveal the secret test key (`sk_test_...`). See
   [Stripe API keys](https://docs.stripe.com/keys).
3. In Workbench → Webhooks, create an event destination for this account,
   using a webhook endpoint with snapshot events (the handler reads
   `data.object`), not thin events or connected-account events. See
   [Stripe webhook setup](https://docs.stripe.com/webhooks).
4. Set the endpoint URL to:

   ```text
   https://staging.knitnprint.com/api/payments/stripe/webhook
   ```

5. Select only the events handled by `backend/src/payments.rs`:

   ```text
   checkout.session.completed
   checkout.session.async_payment_succeeded
   checkout.session.async_payment_failed
   checkout.session.expired
   refund.created
   refund.updated
   refund.failed
   ```

6. Copy the endpoint signing secret (`whsec_...`), which is separate from the
   API key. Keep the API key and endpoint in the same sandbox/test environment.
7. From the repository root, run:

   ```bash
   ./scripts/staging/populate-stripe-secret.sh
   ```

   Paste each credential only at the script's hidden terminal prompts. The
   script writes the JSON value to Secrets Manager without displaying it.

Do not send test webhook events until the service and public staging DNS are
online. Codex verifies secret version metadata next; starting the service is a
separate reviewed Terraform change. The outbound Stripe API version currently
pinned by the application is `2026-02-25.clover`; prefer that version for the
snapshot endpoint if Stripe offers it, and record/review any different version
before testing.

### DNS and SES prerequisites

The prerequisite bootstrap update is complete. Its reviewed plan had SHA-256
`ab6d5e86927fb2bb261a8aaabcfea0f5560162c9e89b4d016013f8a5e25403cb` and
applied with zero additions, one in-place update, and zero deletions. Read-only
AWS verification found `acm:Export = DISABLED` in both the Identity Center
permission set and the generated staging role's `AwsSSOInlinePolicy`. IAM
simulation allowed the exact non-exportable staging certificate request and
implicitly denied the equivalent exportable request.

The regenerated DNS plan then passed both human and JSON inspection. It has
SHA-256 `26fe8f78d00f880d7dcc0d497f49b205f962a2dad5a275b4d146af5993439f0a`,
contains exactly the nine expected creates plus one deferred policy-document
read, and contains no updates, deletions, replacements, secret variables, or
sensitive outputs. Exact-name collision checks through the administrator
profile returned empty results for both ACM certificates, the SES identity and
configuration set, and the SNS topic. The restricted staging role correctly
denies broad list operations; do not broaden it merely for inventory checks.

That exact DNS plan applied successfully with nine additions, no changes, and
no deletions. Live verification confirmed the two ACM certificates are
non-exportable and pending DNS validation, the SES signing hosted zone is
`dkim.amazonses.com`, custom MAIL FROM is pending, configuration-set reputation
metrics are disabled, and the failure event destination points to the scoped
SNS topic. The remote DNS state is encrypted and versioned; its current S3
version ID is `_kJJ9ti8Sn0z3iHng2zLC3LwdtLUVsVS`. A post-apply Terraform plan
reported no changes, and public authoritative DNS lookups found none of the
seven pending record names. The exact Namecheap record table is maintained in
the handoff document.

All seven records were subsequently added with Automatic TTL. Direct queries to
both Namecheap authoritative nameservers returned every expected value. SES
quickly changed the staging identity and DKIM status to `SUCCESS` and enabled
the identity for sending. Follow-up AWS checks on 2026-09-07 confirmed both ACM
certificates are `ISSUED` with successful validation and custom MAIL FROM is
also `SUCCESS`.

## 13. Simplified infrastructure authorization

An action-by-action EC2 policy was drafted for the restricted staging deployer
while preparing the network foundation. The combined policy material approached
9.4 KB before database, ECS, storage, or edge permissions were added. It was
never applied and was removed because the complexity was disproportionate to a
staging environment used by fewer than five people.

The practical model is now:

- use the temporary `knitnprint-administrator` SSO profile for all
  human-reviewed Terraform plans and applies;
- remove the redundant `KnitNPrintStagingDeployer` permission set, assignment,
  state-bucket allowance, and local profile;
- require a saved Terraform plan immediately before each apply;
- keep ECS task roles and other workload identities least privilege;
- later add a small GitHub OIDC role only for routine image publication, ECS
  rollout, migrations, admin asset upload, and CloudFront invalidation.

Production should use the same separation with independent state and resource
names. Do not reproduce the oversized human Terraform policy merely to avoid a
clearly identified temporary administrator session.

The saved cleanup plan had SHA-256
`14cbf15a184009ebb630e0879eceb82aa8bf28e51ea83482ac2e1ed89a321b76`.
It applied exactly as reviewed: zero additions, one state-bucket policy update,
and deletion of the permission-set assignment, inline policy, and permission
set. No bucket, budget, state, DNS, SES, SNS, or certificate resource changed.
Terraform formatting and validation passed for all three roots.
The obsolete local `knitnprint-staging` profile was also removed while
preserving the shared SSO session and `knitnprint-administrator`. Read-only
verification found only the `AdministratorAccess` permission set, only root and
the administrator role in the state-bucket allowlist, and no bootstrap drift.

Terraform now manages the staging state bucket and budget through the remote S3
backend. Terraform left an ignored local backup after migration; it is
owner-readable only and is not the authoritative state.

Before proceeding, capture the local checkpoint:

```bash
git status --short
git log -1 --oneline
terraform version
```

Expected application commit and Terraform version:

```text
e82660e fix: harden deployment runtime images
Terraform v1.16.0
on linux_amd64
```

Any untracked study notes or personal command logs should remain uncommitted unless they are intentionally reviewed and added in a documentation commit.
