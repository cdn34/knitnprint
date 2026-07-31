# KnitPrint

KnitPrint is a craft-led ecommerce platform with a server-rendered public
storefront, a private admin SPA, and a Rust API.

## Requirements

- Node.js 24
- Rust stable
- PostgreSQL 17 (or Docker)

## Local setup

```bash
npm install
cargo build
docker compose up -d
```

Copy `backend/.env.example` to `backend/.env`, export its values, then run each
surface in a separate terminal:

```bash
npm run db:migrate
npm run db:seed
npm run dev:storefront
npm run dev:admin
cargo run -p knitprint-api
```

Create the first staff owner after migrating:

```bash
OWNER_EMAIL=owner@example.com \
OWNER_NAME="Store owner" \
OWNER_PASSWORD="use-a-long-development-password" \
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:create-owner
```

Owner creation is intentionally idempotent by email and requires a password of
at least 12 characters. Use environment or secret-manager injection rather than
placing real credentials in committed files.

- Storefront: http://localhost:3000
- Admin: http://localhost:3001
- API health: http://localhost:8080/api/health
- MinIO console: http://localhost:9101

The API can start without PostgreSQL for development. `/api/health` will remain
healthy while `/api/ready` reports `503` until a database connection is ready.
Production startup requires `DATABASE_URL`.

Local product images use the private `knitprint-media` bucket in MinIO.
`docker compose up -d` creates the bucket automatically. The API defaults to
the local MinIO credentials in development; production requires all five
`S3_*` values shown in `backend/.env.example`.

The admin starts on a session-aware login screen and proxies `/api` requests to
the local Rust API. Both processes must be running. After signing in, refreshing
the browser preserves the server-side session; use the sign-out button beside
the staff profile to revoke it.

Failed admin sign-ins are limited to five attempts per email in a rolling
15-minute window. Run authentication cleanup from a scheduler (daily is
appropriate for most installations):

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:cleanup-sessions
```

The command removes expired sessions, revoked sessions older than seven days,
and stale login-attempt records. Set `SESSION_RETENTION_DAYS` to a value from 1
to 365 to change the revoked-session retention period.

Clean abandoned product-image uploads from PostgreSQL and MinIO on the same
daily schedule:

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:cleanup-media
```

Pending uploads older than 24 hours are removed by default. Set
`MEDIA_PENDING_MAX_HOURS` from 1 to 168 to adjust that window. Cleanup claims
at most 100 records per run with row locking, removes storage objects first,
and retains an immutable system audit entry.

## API contract

The Rust routes and response types are the source of truth for OpenAPI. Regenerate
the checked-in contract and shared TypeScript types after changing an endpoint:

```bash
npm run api:generate
```

The OpenAPI document is written to `openapi/knitprint.json`, while the generated
schema types and reusable fetch client live in `packages/api-client`. A running
API also serves the contract from `/api/openapi.json`.

## Checks

```bash
npm run typecheck
npm run build
npm run api:check
npm run test:e2e
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run test:e2e:admin
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Install the local Chromium runtime once before running browser tests:

```bash
npx playwright install chromium
```

Brand derivatives are generated deterministically from `images/logo.png`:

```bash
npm run assets:brand
```
