# Project handoff

Last updated: 2026-08-02

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
  search, archival, and catalog audit history;
- admin product workspace with search, draft creation, first-variant pricing,
  inline preview, publishing, and archival controls;
- Playwright coverage proving draft creation, preview, search, publication, and
  public API visibility;
- private MinIO bucket bootstrap with local admin-origin CORS;
- five-minute presigned JPEG, PNG, and WebP uploads limited to 10 MB;
- server-generated quarantine object keys with no storage credentials exposed
  to the browser;
- completion verification against S3 object size and declared media type;
- transactional product attachment, ordering, alt text, and media audit record;
- admin image selection and immediate product/preview rendering;
- Playwright coverage performing a real browser-to-MinIO upload and completion;
- server-rendered storefront catalog data with a graceful API-unavailable state;
- responsive published-product grid and client-side catalog filtering;
- server-rendered product detail route with live price, variant, and SKU data;
- desktop and mobile Playwright coverage for search and product navigation;
- ordered ready-media metadata in admin and public product responses;
- stable `/api/media/{id}` delivery restricted to media attached to active
  products, with immutable caching and object content types;
- persisted admin thumbnails and previews after reload;
- uploaded product photography on storefront cards and detail pages;
- end-to-end verification of media metadata, content headers, and persistence;
- image signature decoding with a 40-million-pixel safety limit;
- normalized WebP thumbnail (320px), card (900px), and detail (1600px)
  variants stored separately from quarantine originals;
- variant-specific immutable media URLs used by each admin/storefront context;
- real MinIO verification of generated objects, dimensions, byte sizes, and
  WebP delivery headers.
- concurrency-safe abandoned-upload claiming with `FOR UPDATE SKIP LOCKED`;
- idempotent removal of quarantine originals and partially generated variants;
- database deletion only after successful storage cleanup, with failed work
  retained for retry;
- immutable `media.abandoned_cleanup` system audit entries and configurable
  1–168 hour retention;
- live PostgreSQL/MinIO verification that stale records and objects are removed.
- capability-protected category creation, ordered category assignment, and
  additional-variant creation for existing products;
- transactional audit records for category creation, variant addition, and
  category assignment;
- typed category and variant-management client methods and an admin editor that
  shows existing variants, creates categories, and assigns them to products;
- PostgreSQL and Playwright lifecycle coverage for the richer product editor.
- public discovery of categories that contain active products and server-side
  product filtering by category slug;
- generated client support for public categories and filtered product lists;
- live category cards on the storefront home and server-rendered collection
  pages with product grids, metadata, empty states, and responsive styling;
- PostgreSQL and desktop/mobile Playwright coverage for collection discovery,
  filtering, and navigation.

Phase 2 is complete.

The first Phase 3 inventory slice is implemented:

- automatically provisioned per-variant inventory with available, reserved,
  committed, and low-stock threshold quantities;
- database constraints preventing negative quantities and triggers preventing
  updates or deletion of immutable inventory movements;
- row-locked, reason-required manual adjustment transactions with overflow and
  negative-stock protection;
- capability-protected inventory list, movement history, and adjustment APIs
  with independent audit events;
- available quantity and low-stock state in admin and public variant contracts;
- a dedicated responsive Inventory page with low-stock indicators, adjustment
  controls, and actor-attributed movement history;
- PostgreSQL coverage for authorization, negative-stock rejection, movement
  immutability, audit records, and public availability, plus a real Playwright
  admin adjustment workflow.

The transactional inventory lifecycle is also implemented:

- reusable availability, reserve, release, commit, and adjustment service
  operations for later cart and order flows;
- row-locked mutations that serialize competing operations for each variant;
- checked arithmetic and explicit insufficient-available/reserved errors that
  preserve all non-negative inventory invariants;
- reservation, release, and commitment entries in the immutable movement
  history, including system-attributed reasons;
- PostgreSQL lifecycle coverage for reserve/release/commit behavior and error
  paths;
- a 12-way contention test proving that only five reservations can succeed
  when five units are available, with exactly five corresponding movements.

Storefront availability is implemented:

- concise in-stock, low-stock, and sold-out messaging on home and collection
  product cards;
- product detail variant controls that keep sold-out options visible but
  disabled and default to the first available variant;
- live selected-variant price, SKU, availability message, and future cart
  action state;
- accessible native radio semantics, keyboard focus, live status messaging,
  WCAG AA color contrast, and responsive layouts without viewport overflow;
- desktop/mobile storefront coverage for stock rendering and detail-page
  accessibility;
- a deterministic PostgreSQL-backed browser lifecycle covering a sold-out
  default, a low-stock option, an in-stock option, price/SKU changes, and mobile
  layout;
- cancellation-safe admin product cache updates so concurrent initial loading
  cannot temporarily hide a newly created draft.

Operational inventory visibility is implemented:

- live dashboard totals for available units, reserved units, tracked variants,
  low-stock variants, and out-of-stock variants;
- an actionable dashboard list ordered by available quantity with direct
  navigation into Inventory;
- inventory search across product, variant, and SKU plus all, attention,
  out-of-stock, and healthy stock-state filters with live counts;
- shared query invalidation when products and variants provision inventory, so
  dashboard and inventory data cannot become stale;
- PostgreSQL-backed browser coverage for metrics, navigation, search, every
  stock filter, accessibility, and mobile overflow containment.

Phase 3 is complete. Phases 0 through 3 now form the first working milestone:
secure staff can publish catalog products, manage media and stock, prevent
overselling, and expose an availability-aware public storefront.

The first Phase 4 customer and address foundation is implemented:

- write-only guest contact and delivery-address capture that remains separate
  from staff authentication;
- hashed idempotency keys and transaction-scoped advisory locking so retries
  return the original customer/address identifiers without duplicate records;
- database constraints for customer type, contact lengths, address type,
  country code, retention deadlines, and referential cleanup;
- a 24-month guest retention deadline, with expired or anonymized records
  excluded from all application reads;
- capability-protected, bounded customer search and detail APIs;
- immutable audit events for guest creation, staff list/search access, and
  private detail access without storing search terms or copied personal data;
- generated OpenAPI/TypeScript contracts and client methods;
- a permission-gated, responsive admin Customers workspace with search,
  contact and address detail, retention visibility, and an order-history
  placeholder;
- isolated PostgreSQL coverage for validation, idempotency, explicit
  `customers.read` authorization, privacy audits, expiry, and anonymization;
- Playwright coverage for customer search/detail, WCAG A/AA checks, and mobile
  overflow containment.

The registered customer account slice is also implemented:

- separate storefront customer identities with normalized unique email
  addresses and Argon2 password hashes;
- registration, login, logout, current-profile, and owned address-creation
  APIs with generated OpenAPI and TypeScript client support;
- opaque 30-day customer sessions stored only as SHA-256 token hashes, using
  HTTP-only, `SameSite=Lax` cookies and secure cookies in production;
- customer/staff authentication isolation, durable session revocation,
  private no-store responses, and customer-attributed audit events;
- rolling registered-customer retention renewal on authenticated activity;
- per-email failed-login limiting at five attempts in 15 minutes;
- ownership-scoped address reads and writes that do not expose another
  customer's records;
- an accessible responsive `/account` storefront experience for registration,
  login, persistent profile/address display, address creation, and logout;
- a real account link in storefront navigation and a development `/api` proxy
  for same-origin session-cookie behavior;
- isolated PostgreSQL lifecycle coverage for validation, password/session
  storage, auth isolation, ownership, throttling, logout, expiry, and auditing;
- desktop/mobile browser coverage for registration, reload persistence,
  address creation, logout/login, WCAG A/AA, and viewport overflow.

Phase 4 remains in progress. Production deployment must route storefront
`/api/*` requests to the Rust API on the same public origin, preserving cookies
and `Set-Cookie` headers over HTTPS; the Vite proxy only covers development.

Before production customer traffic:

- implement registered-email verification and password recovery;
- add perimeter/global/IP throttling and make login limiting robust under
  concurrent attempts rather than relying only on the current per-email
  database counter;
- add irreversible retention cleanup that anonymizes expired customer contact
  fields, deletes or anonymizes owned addresses, and removes expired sessions;
- bind guest capture to the checkout/order lifecycle and its anti-abuse
  controls.

The complete Rust, API-contract, TypeScript, production-build, 16-test
storefront Playwright, and four-test PostgreSQL-backed admin Playwright suites
passed on 2026-08-02.

## Environment notes

- No deployment has been performed.
- PostgreSQL and MinIO were verified with Docker Compose and left running for
  the next feature slice.
- Work is committed in focused changes; preserve the user's untracked
  `commands.md`.
- Generated public logo assets are reproducible with `npm run assets:brand`;
  the source at `images/logo.png` remains authoritative and untouched.
