# Local setup

Run everything from the repository root:

```bash
cd /home/carlosn34/projects/test-p
```

## Switching from MinIO

SeaweedFS uses a new `seaweedfs_data` volume; existing MinIO objects do not
appear automatically. Keep the old volume if its uploads matter and copy
objects through the S3 API from a working MinIO instance. Existing database
media records still refer to those objects until copied. Start the replacement
with `docker compose up -d seaweedfs`. The endpoint, bucket, and S3 credentials
in `backend/.env.example` are unchanged. The SeaweedFS admin UI uses
`knitnprint` / `knitnprint-local`.

## First-time setup

### If Docker Desktop stalls

If container startup and `docker inspect` both hang, restart Docker Desktop
with `docker desktop restart`, then retry
`docker compose up -d --wait seaweedfs`.

SeaweedFS can also run natively. Download version 4.48 for your operating
system from [the official releases](https://github.com/seaweedfs/seaweedfs/releases/tag/4.48),
extract `weed`, and keep this running in a separate terminal:

```bash
SEAWEEDFS_BIN=/absolute/path/to/weed bash scripts/start-local-storage.sh
```

The native launcher uses the same S3 endpoint, admin UI, bucket, credentials,
and CORS origins as Compose. Data is stored in `.local/seaweedfs` (override with
`SEAWEEDFS_DATA_DIR`). Native and Docker data stores are separate; run only one
at a time. PostgreSQL still needs Docker or a separate local installation.

### Install and bootstrap

```bash
npm ci
cargo build
test -f backend/.env || cp backend/.env.example backend/.env
docker compose up -d

set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint

npm run db:migrate
npm run db:seed

OWNER_EMAIL=owner@knitnprint.local \
OWNER_NAME="Local owner" \
OWNER_PASSWORD=local-development-passphrase \
npm run admin:create-owner
```

If this checkout reuses a Docker volume created before the `KnitNPrint` rename,
or one containing migrations from another branch, recreate the disposable local
database and SeaweedFS volumes before running the setup commands above:

```bash
# Deletes local development data only. It does not affect AWS staging.
docker compose down --volumes
docker compose up -d
```

Install the browser used by Playwright once:

```bash
npx playwright install chromium
```

## Start the application

Keep these commands running in separate terminals.

Infrastructure:

```bash
docker compose up -d
docker compose ps
```

API:

```bash
set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint
export EMAIL_DELIVERY=development
cargo run -p knitnprint-api --bin knitnprint-api
```

Storefront:

```bash
npm run dev:storefront -- --host 127.0.0.1
```

Admin:

```bash
npm run dev:admin -- --host 127.0.0.1
```

Open:

- Storefront: http://127.0.0.1:3000
- Customer account: http://127.0.0.1:3000/account
- Admin: http://127.0.0.1:3001
- API readiness: http://127.0.0.1:8080/api/ready
- SeaweedFS admin UI: http://127.0.0.1:9101

Local admin credentials:

```text
Email: owner@knitnprint.local
Password: local-development-passphrase
```

## Storage integration check

```bash
docker compose up -d --wait seaweedfs
cargo test -p knitnprint-api live_seaweedfs_round_trip -- --ignored
```

This verifies browser CORS preflight, signed PUT/GET, private access, metadata,
SDK writes, and deletion against the local media bucket.

## Quick checks

```bash
curl --fail-with-body http://127.0.0.1:8080/api/health
curl --fail-with-body http://127.0.0.1:8080/api/ready
curl --fail-with-body http://127.0.0.1:3000/health
curl --fail-with-body http://127.0.0.1:3000/
curl --fail-with-body http://127.0.0.1:3001/
```

## Automated checks

Load `backend/.env` before running database-backed tests:

```bash
set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint

npm run typecheck
npm run build
npm run api:check
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Browser tests:

```bash
npm run test:e2e

E2E_OWNER_EMAIL=owner@knitnprint.local \
E2E_OWNER_PASSWORD=local-development-passphrase \
npm run test:e2e:admin
```

Run only the customer account and local-email test:

```bash
npx playwright test tests/e2e/account.spec.ts --project=desktop-chromium
```

Build the storefront image used by ECS:

```bash
docker build \
  --file apps/storefront/Dockerfile \
  --tag knitnprint-storefront:local-test \
  .
```

## Test email locally

With `EMAIL_DELIVERY=development`, email stays in the API's in-memory mailbox
and is never sent externally. Register an account at
http://127.0.0.1:3000/account, then inspect its latest verification email:

```bash
TEST_EMAIL='customer@example.test'

curl --fail-with-body --silent --show-error \
  --get http://127.0.0.1:8080/api/development/emails/latest \
  --data-urlencode "to=${TEST_EMAIL}" \
  --data-urlencode 'kind=email_verification'
```

Request and inspect a password-reset email:

```bash
curl --fail-with-body --silent --show-error \
  --header 'content-type: application/json' \
  --data "{\"email\":\"${TEST_EMAIL}\"}" \
  http://127.0.0.1:8080/api/account/password/forgot

curl --fail-with-body --silent --show-error \
  --get http://127.0.0.1:8080/api/development/emails/latest \
  --data-urlencode "to=${TEST_EMAIL}" \
  --data-urlencode 'kind=password_reset'
```

Each response contains an `action_url` to open in the browser. The mailbox is
cleared whenever the API restarts.

## Send a real email through SES from local development

On each new computer, install AWS CLI v2 and configure the administrator SSO
profile once. The setup command is `aws configure sso`; `aws sso login` only
authenticates an already configured profile. Do not run either command with
`sudo`.

```bash
aws --version
aws configure sso --profile knitnprint-administrator
```

Use these values in the configuration wizard:

```text
SSO session name (Recommended):                    knitnprint
SSO start URL [None]:                               <AWS access portal URL>
SSO region [None]:                                  us-east-1
SSO registration scopes [sso:account:access]:       sso:account:access
AWS account/role:                                   AdministratorAccess
Default client Region [None]:                       eu-west-1
CLI default output format (json if not specified):  json
Profile name [suggested-name]:                      knitnprint-administrator
```

Get the start URL from **IAM Identity Center → Settings → AWS access
portal URL**; do not use the ordinary AWS Console URL. See
[`guides/production/aws-identity-and-staging-credentials.md`](guides/production/aws-identity-and-staging-credentials.md#initial-administrator-cli-profile)
for the complete identity setup.

After that one-time setup, stop the API, authenticate to AWS, and restart it
with SES enabled. Repeat `aws sso login` whenever the cached SSO session
expires. Keep `APP_ENV=development`; staging mode rejects local HTTP and SeaweedFS
settings.

```bash
aws sso login --profile knitnprint-administrator

aws sts get-caller-identity \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager

set -a
source backend/.env
set +a

export APP_ENV=development
export EMAIL_DELIVERY=ses
export EMAIL_FROM=no-reply@staging.knitnprint.com
export AWS_REGION=eu-west-1
export AWS_PROFILE=knitnprint-administrator
export SES_CONFIGURATION_SET=knitnprint-staging-transactional
export SES_TEST_EMAIL='your-email@example.com'
export EMAIL_RECIPIENT_ALLOWLIST="${SES_TEST_EMAIL}"

cargo run -p knitnprint-api --bin knitnprint-api
```

Register `SES_TEST_EMAIL` at http://127.0.0.1:3000/account to send a real
verification message. If SES is still in sandbox mode, the recipient must also
be verified in SES. The development-mailbox endpoint is unavailable in SES
mode.

## Deliver queued order emails

Account verification and password-reset messages are sent immediately by the
API. Order-confirmation and fulfillment messages use the PostgreSQL outbox and
require the one-shot delivery worker:

```bash
set -a
source backend/.env
set +a
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint

npm run admin:deliver-notifications
```

Run it after an order or fulfillment event, or invoke it once per minute while
testing. For a real inbox delivery, run it with the SES variables from the
previous section. If the worker runs in a new terminal, export those variables
again because shell environment variables are not shared between terminals. In
development-mailbox mode the worker uses its own temporary in-memory mailbox,
so its messages are not visible through the separately running API's
development-email endpoint.

## Database shell

```bash
docker compose exec postgres psql -U knitnprint -d knitnprint
```

Exit with `\q`.

## Stop

Stop the API, storefront, and admin with `Ctrl+C`, then preserve local data:

```bash
docker compose down
```

Delete all local PostgreSQL and SeaweedFS data only when a full reset is intended:

```bash
docker compose down --volumes
```
