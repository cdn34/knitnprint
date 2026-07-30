# Project handoff

Last updated: 2026-07-29

## Goal

Build KnitPrint as a deliberately simple ecommerce platform with:

- a server-rendered public storefront;
- a private admin SPA;
- a Rust/Axum API and PostgreSQL;
- a warm visual identity derived from `images/logo.png`;
- independently delivered vertical feature slices.

The complete architectural and delivery plan is in `IMPLEMENTATION_PLAN.md`.

## Current implementation

Phase 0 has started and the runnable application foundation is present:

```text
apps/storefront/  TanStack Start SSR storefront
apps/admin/       React/Vite admin SPA
backend/          Rust/Axum API
images/           original KnitPrint source logo
compose.yaml      PostgreSQL and MinIO for local development
```

The storefront includes:

- responsive branded navigation and footer;
- an SSR homepage with hero, product previews, collections, story, and shopping
  reassurance;
- semantic brand tokens derived from the logo;
- responsive desktop and mobile layouts;
- visible keyboard focus, skip navigation, and reduced-motion behavior;
- CSS-rendered product preview forms until catalog media exists.

The admin includes:

- the planned primary navigation;
- a responsive foundation dashboard;
- explicit placeholders for features that do not exist yet.

The API includes:

- structured tracing and request IDs;
- validated environment, host, port, and production database configuration;
- `GET /api/health`;
- database-aware `GET /api/ready`;
- `GET /api/openapi.json` and a checked-in OpenAPI contract;
- consistent JSON 404 and readiness errors;
- graceful shutdown and bounded PostgreSQL pooling.

The Phase 0 infrastructure now also includes:

- an idempotent forward SQL migration and separate migrate/seed commands;
- a shared generated TypeScript schema and reusable fetch client;
- deterministic API contract generation without third-party generator
  dependencies;
- seven backend tests covering configuration and HTTP contracts;
- CI for TypeScript, production builds, Rust formatting, Clippy, tests,
  PostgreSQL migrations/seeding, contract drift, and production npm audits.
- Playwright coverage at desktop and mobile widths for the storefront shell,
  viewport overflow, keyboard skip navigation, and automated WCAG A/AA checks;
- deterministic optimized WebP wordmark and compact-mark generation from the
  authoritative source logo.

## Verified

The following passed on 2026-07-29:

```bash
npm run typecheck
npm run build
npm run api:check
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Runtime smoke checks also verified:

- `/` returns meaningful server-rendered HTML and KnitPrint metadata;
- `/api/health` returns `200` and a JSON health payload;
- `/api/ready` returns a structured `503` when `DATABASE_URL` is absent.
- PostgreSQL 17 accepts the migration and idempotent seed;
- the SQLx migration record and development seed metadata are present;
- `/api/ready` returns `200` against the containerized database;
- MinIO responds successfully to its live health check;
- all eight storefront browser/accessibility checks pass on desktop and mobile
  Chromium.

## Local development

Requirements:

- Node.js 24;
- Rust stable;
- Docker for the local PostgreSQL and MinIO services.

Setup and start commands are documented in `README.md`.

## Phase 1 progress

The staff-authentication backend is implemented:

- staff users with `owner` and `staff` roles;
- explicit capability definitions and per-staff grants;
- Argon2 password hashing;
- initial-owner creation command with password-length validation;
- opaque 256-bit session tokens stored only as SHA-256 hashes;
- HTTP-only, strict same-site cookies with secure cookies in production;
- login, logout, and current-profile endpoints;
- disabled-user and expired/revoked-session enforcement;
- owner capability expansion;
- immutable audit log with transactional login and logout events;
- reusable authenticated-staff extraction and per-operation capability checks;
- owner-only staff listing, creation, and disabling;
- immediate session revocation when a staff account is disabled;
- self-disable and last-owner safeguards;
- database-backed five-attempt login throttling with a 15-minute window;
- repeatable cleanup for expired sessions, retained revoked sessions, and stale
  login-attempt records;
- OpenAPI and shared TypeScript client methods for auth and staff management.

The admin authentication experience is also implemented:

- session check before rendering private operations UI;
- accessible branded login and service-unavailable states;
- TanStack Query session lifecycle;
- same-origin Vite proxy for secure cookie behavior in development;
- protected dashboard populated from the authenticated staff profile;
- owner staff-management UI with granular capability assignment;
- logout and session revocation;
- Playwright coverage for login, persistence across reload, logout, and WCAG
  A/AA checks on both login and dashboard;
- Playwright coverage for owner staff creation, granular capability assignment,
  audit-reason capture, and disabling;
- isolated PostgreSQL-backed admin authentication coverage in CI.

Local PostgreSQL verification covers owner creation, login, profile retrieval,
all owner capabilities, staff creation, capability denial, disabling, immediate
session revocation, and audit actor/reason records. Eleven Rust unit tests and
one isolated PostgreSQL integration test pass.

## Next implementation slice

Phase 1 is complete.

## Phase 2 progress

The catalog foundation is implemented:

- normalized products, variants, categories, category relationships, media
  assets, and product/variant media relationships;
- database-enforced draft, active, and archived states;
- integer minor-unit prices and explicit uppercase currencies;
- unique product slugs and variant SKUs;
- generated PostgreSQL full-text search document with a GIN index;
- capability-protected admin list, detail, create, and status operations;
- public listing, detail, and search restricted to active products;
- transactional audit records for product creation and status changes;
- OpenAPI definitions and typed TypeScript client methods;
- isolated PostgreSQL coverage for permissions, draft privacy, publication,
  search, archival, and catalog audit history.

Continue Phase 2 with:

1. admin product list/editor and preview;
2. presigned MinIO media upload and completion verification;
3. product image assignment, ordering, and alt text;
4. storefront product listing, detail, and search UI.

## Environment notes

- No deployment has been performed.
- PostgreSQL and MinIO were verified with Docker Compose and left running for
  the next feature slice.
- Work is committed in focused changes; preserve the user's untracked
  `commands.md`.
- Generated public logo assets are reproducible with `npm run assets:brand`;
  the source at `images/logo.png` remains authoritative and untouched.
