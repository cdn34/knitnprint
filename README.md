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
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
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
Production startup requires `DATABASE_URL` and does not run migrations. Apply
them first with the separate migration credential in `MIGRATION_DATABASE_URL`.

Local product images use the private `knitprint-media` bucket in MinIO.
`docker compose up -d` creates the bucket automatically. The API defaults to
the local MinIO credentials in development; production requires all five
`S3_*` values shown in `backend/.env.example`.

The admin starts on a session-aware login screen and proxies `/api` requests to
the local Rust API. Both processes must be running. After signing in, refreshing
the browser preserves the server-side session; use the sign-out button beside
the staff profile to revoke it.

The storefront also proxies `/api` to the local Rust API and provides optional
registered customer accounts at http://localhost:3000/account. Customers can
register, sign in, preserve their session across reloads, view their contact
details and owned delivery addresses, add an address, and sign out. Customer
sessions and staff sessions are separate, and guest checkout data remains
independent of registered accounts.

The storefront cart at http://localhost:3000/cart persists for 30 days. Its
opaque browser token is stored only as a SHA-256 hash, every mutation requires
an idempotency key, and prices, publication state, and available stock are
reconciled by the API. Delivery capture creates or updates the guest
customer/address attached to that cart; authenticated carts reuse the
registered customer identity. Adding an item does not reserve stock.

Customers can apply active fixed-amount or percentage discount codes in the
cart. The API controls code normalization, dates, minimum order amounts,
currency, and global or per-customer usage limits. Checkout recalculates the
discount while locking its usage counter, then stores an immutable rule and
amount snapshot on the order. A later discount change therefore cannot alter a
historical order, and removing or rejecting a code never changes the cart's
items or base prices. Staff with `discounts.manage` can create, enable, disable,
and inspect codes in the admin Discounts workspace; every change requires an
audit reason. Usage is counted when an order is created and is not restored by
later cancellation or refund.

Staff with `settings.manage` can maintain the non-secret store identity,
currency, flat-rate shipping zones and methods, and destination tax rules in
the admin Settings workspace. Explicit country zones take precedence over the
single worldwide fallback. The server applies discounts before shipping, then
calculates exclusive tax on discounted merchandise plus shipping. Cart totals
are estimates from the same server calculator used at checkout; the selected
shipping method, destination rule, tax base, rate, and amounts are stored as
immutable order snapshots.

Tax calculation is disabled by default and no production rate is seeded. When
enabled, checkout requires a matching active destination rule; use an explicit
zero-percent rule where that behavior is intentional. Confirm registration,
place-of-supply, product classification, shipping-taxability, rounding,
invoicing, and rate requirements with a qualified adviser for every served
jurisdiction before enabling production tax collection. The Settings page also
reports whether database, media, email, and payment integrations are configured,
but never accepts or displays their credentials.

The admin Dashboard is a live, capability-aware operational projection. Staff
with order access see paid orders awaiting fulfillment, recent orders, current
payment failures, recent refunds, and order/revenue totals; staff with inventory
access see low-stock variants. Every queue row links directly to its order or
variant. Lists are bounded to eight records and refetch whenever the Dashboard
is opened. “Today” begins at 00:00 UTC. Gross revenue is captured payment value
in the configured store currency, refunds include succeeded refunds in that
currency, and net revenue is gross minus refunds. Counts may include orders in
other currencies where the on-screen metric definition says so.

Creating an order reserves stock. Development keeps the audited manual-payment
path enabled. Hosted Stripe Checkout is enabled only when all of these values
are present:

```bash
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
STOREFRONT_BASE_URL=http://localhost:3000
```

Production requires an `sk_live_` key and an HTTPS storefront URL. Configure a
Stripe webhook endpoint at `/api/payments/stripe/webhook` for
`checkout.session.completed`, `checkout.session.async_payment_succeeded`,
`checkout.session.async_payment_failed`, `checkout.session.expired`,
`refund.created`, `refund.updated`, and `refund.failed`, using API version
`2026-02-25.clover`. The API verifies Stripe's signature against the
untouched request body with a five-minute timestamp tolerance. Only a verified
paid event confirms an order and commits inventory; failed or expired checkout
releases the reservation.

Staff with `orders.refund` can cancel an unpaid, unfulfilled order or create a
server-priced partial/full refund from the admin order detail. Every operation
requires an idempotency key and reason; refunds also record an explicit
restocking decision and support private internal notes. Stripe refunds use the
verified payment-intent reference; manual refunds remain available only in
development/test.

Registration sends a 24-hour email-verification link, and forgotten-password
requests send a one-hour single-use reset link. Development and test processes
keep these messages in an in-memory mailbox used by browser automation; no
external email is sent by default. To send real email through SES while keeping
the rest of the application in development mode, export:

```bash
APP_ENV=development
EMAIL_DELIVERY=ses
STOREFRONT_BASE_URL=http://127.0.0.1:3000
EMAIL_FROM=accounts@example.com
AWS_REGION=eu-west-1
AWS_PROFILE=knitprint-development
# Optional: SES_CONFIGURATION_SET=knitprint-transactional
```

`AWS_PROFILE` is intended for local development and may be omitted when the
standard AWS credential chain already resolves a workload role or environment
credentials. Production uses SES by default and rejects
`EMAIL_DELIVERY=development`; its minimum email configuration is:

```bash
APP_ENV=production
STOREFRONT_BASE_URL=https://shop.example.com
EMAIL_DELIVERY=ses
EMAIL_FROM=accounts@example.com
AWS_REGION=eu-west-1
# Optional: SES_CONFIGURATION_SET=knitprint-transactional
```

Use the standard AWS SDK credential chain (prefer an instance/task/runtime
role) with `ses:SendEmail` permission. The From address or its domain must be a
verified SES identity in the configured region, and the SES account must be
able to send to unverified customer recipients rather than remaining limited
to sandbox destinations. Verification and reset secrets are 256-bit,
single-use, expire server-side, and are stored only as SHA-256 hashes. A
successful reset revokes every existing customer session.

The Vite proxies are development-only. In production, the public web server or
ingress must route `/api/*` to the Rust API under the same public origin as the
storefront. It must preserve request cookies and API `Set-Cookie` headers, and
production must use HTTPS so secure session cookies work. Do not configure the
browser to call a separate API origin unless the cookie and CSRF design is
intentionally revised.

Production also requires `WEB_ORIGINS` containing the exact HTTPS storefront
and admin origins. Unsafe browser requests from any other origin are rejected,
credentialed CORS is restricted to that list, request bodies are capped at 1
MiB, and API responses receive defensive browser headers. Registration,
verification-email, and password-reset requests have persistent hashed
account/IP/global abuse limits in addition to the login limiter.

Uploaded media remains quarantined until its object metadata, declared type,
file signature, decode limits, and malware scan pass. Production requires a
ClamAV-compatible TCP INSTREAM service in `MEDIA_SCANNER_ADDRESS`; scanner
failure or timeout fails closed and cannot publish the object. When custom
local `S3_ACCESS_KEY_ID`/`S3_SECRET_ACCESS_KEY` values are absent, production S3
uses the standard AWS SDK credential chain so a workload role can be used.

Staff and customer sign-ins share a PostgreSQL-backed limiter with independent
scopes. Each account allows five failed attempts in 15 minutes, each client IP
allows 60 login requests in five minutes, and each authentication scope allows
1,000 login requests per minute. Account advisory locks make the five-attempt
boundary exact under concurrent requests; short transactions serialize only
the IP/global counter updates. Bucket identifiers are stored only as SHA-256
hashes.

By default client IP limits use the direct TCP peer. If a trusted ingress
overwrites `X-Forwarded-For` and the API cannot be reached around that ingress,
set `TRUST_PROXY_HEADERS=true` to use its first forwarded address. Never enable
this when clients can supply or preserve that header themselves. An edge or
ingress request limit is still recommended to reject abusive traffic before it
consumes application or database resources.

Run authentication cleanup from a scheduler (daily is appropriate for most
installations):

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:cleanup-sessions
```

The command removes expired sessions, revoked sessions older than seven days,
and stale hashed staff login-limit buckets. Set `SESSION_RETENTION_DAYS` to a
value from 1 to 365 to change the revoked-session retention period. Customer
cleanup removes stale customer login-limit buckets.

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

Run customer-retention cleanup from a daily scheduler as well:

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:cleanup-customers
```

Each run claims at most 100 expired customers with row locking, irreversibly
replaces their contact fields, deletes their addresses and account credentials,
and records a non-personal immutable audit event. It also removes expired
customer sessions, revoked customer sessions older than seven days, and stale
login-attempt counters. Set `CUSTOMER_CLEANUP_BATCH_SIZE` from 1 to 1000 and
`CUSTOMER_SESSION_RETENTION_DAYS` from 1 to 365 to adjust those bounds. The same
retention removes expired or old-used account-action tokens. Schedule repeated
runs if the expired backlog can exceed the configured batch size.

Remove expired disposable carts on the same daily schedule:

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:cleanup-carts
```

Cleanup claims and removes at most 100 expired carts per run by default, using
row locks that are safe for concurrent schedulers. Set
`CART_CLEANUP_BATCH_SIZE` from 1 to 1000 to change that bound. Customer and
address retention remains governed independently by customer cleanup.

Run abandoned-payment cleanup frequently (for example, every five minutes):

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:cleanup-payments
```

Hosted Checkout sessions last 35 minutes. Cleanup adds a one-hour grace period
for delayed webhooks, then claims at most 100 expired attempts with row locks,
cancels the unpaid order, releases reserved inventory, and appends payment,
order-timeline, and audit history. Set `PAYMENT_CLEANUP_BATCH` from 1 to 1000 to
change the bound; repeated and concurrent runs are safe.

Fulfillment and order emails use a durable PostgreSQL outbox. Run its delivery
worker continuously or once per minute:

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npm run admin:deliver-notifications
```

The worker claims at most 25 due jobs with `FOR UPDATE SKIP LOCKED`, records
every delivery attempt, and retries failures with bounded exponential backoff
for up to eight attempts. Set `NOTIFICATION_BATCH_SIZE` from 1 to 100 to change
the batch. A stale processing claim becomes eligible again after 15 minutes.
Order payment and fulfillment transactions only enqueue work; SES failure never
rolls back or duplicates the commercial operation. Development delivery uses
the in-memory mailbox unless `EMAIL_DELIVERY=ses` is selected, while production
always uses the configured SES sender.

Run the operational backlog check from monitoring at least every five minutes:

```bash
DATABASE_URL=postgres://knitprint-runtime@localhost:5432/knitprint \
npm run admin:check-operations
```

It writes a single JSON status report and exits nonzero for terminal/stale
notification work, overdue payment cleanup, stale or infected media, expired
carts, or expired customer-retention records. The full production gate,
least-privilege PostgreSQL and AWS policy templates, alert thresholds, secret
rotation order, and backup/restore drill are documented in
`docs/operations/production-readiness.md`.

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

The registered-customer integration and browser lifecycle require migrated
PostgreSQL and a database-connected API. Keep the API command from the local
setup section running, then use:

```bash
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
cargo test --test customer_account_lifecycle
DATABASE_URL=postgres://knitprint:knitprint@localhost:5432/knitprint \
npx playwright test tests/e2e/account.spec.ts
```

The application-side security and operational hardening milestone is
implemented. Production launch still requires completing the infrastructure
deployment gate and recording evidence in the target AWS, PostgreSQL, Stripe,
SES, scanner, monitoring, and backup environments. Production email also
depends on a verified SES identity and production sending access. Configure an
edge request limit even though sensitive account endpoints enforce persistent
hashed account, IP, and global limits.

Brand derivatives are generated deterministically from `images/logo.png`:

```bash
npm run assets:brand
```
