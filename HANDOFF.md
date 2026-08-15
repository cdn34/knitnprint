# Project handoff

Last updated: 2026-08-15

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
- hashed per-account failed-login limiting at five attempts in 15 minutes;
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

Customer retention cleanup is implemented:

- bounded 100-record batches that claim expired customers with
  `FOR UPDATE SKIP LOCKED` for safe concurrent scheduling;
- irreversible replacement of retained customer contact fields and deletion
  of owned addresses, account credentials, and customer sessions;
- removal of globally expired sessions, old revoked sessions, and stale
  customer login-limit buckets;
- retained non-personal customer identities for future commercial history and
  immutable, non-PII `customer.retention_anonymize` audit events;
- configurable batch and revoked-session retention bounds with repeat-safe
  behavior;
- isolated PostgreSQL coverage proving expired guest and registered cleanup,
  active-customer preservation, dependent-record removal, auditability, and
  idempotent reruns.

Registered-email verification and password recovery are implemented:

- automatic 24-hour verification links and authenticated resend with a
  five-minute suppression window;
- generic forgotten-password responses, one-hour reset links, and matching
  resend suppression that do not disclose whether an account exists;
- opaque 256-bit single-use action tokens stored only as SHA-256 hashes, with
  expiry and durable consumption enforced by PostgreSQL;
- password replacement that also verifies email ownership, invalidates account
  action tokens, revokes every customer session, and clears the current cookie;
- AWS SES v2 production delivery through the standard AWS credential chain,
  optional configuration-set support, validated HTTPS action URLs, and a
  development-only in-memory mailbox;
- accessible account verification, resend, forgotten-password, and new-password
  storefront states with generic privacy-preserving messaging;
- isolated PostgreSQL and real browser coverage for hashing, expiry/single-use
  behavior, unknown-address privacy, resend suppression, verification,
  password replacement, session revocation, accessibility, and mobile layout.

Concurrent login throttling is implemented for staff and customers:

- one shared PostgreSQL limiter with isolated staff/customer scopes and hashed
  account, client-IP, and global bucket identifiers;
- exact five-failure account limits under contention using transaction-scoped
  advisory locks without serializing unrelated password verification;
- short serialized counter transactions enforcing 60 login requests per IP in
  five minutes and 1,000 requests per authentication scope per minute;
- successful login cleanup of the relevant account-failure bucket and bounded
  `Retry-After` responses for account, IP, and global limits;
- direct TCP peer addresses by default, with explicit `TRUST_PROXY_HEADERS`
  opt-in only for an ingress that overwrites `X-Forwarded-For` and prevents
  direct API access;
- cleanup support for stale hashed buckets and PostgreSQL contention coverage
  proving exactly five ordinary failures followed by seven concurrent 429s,
  plus the independent 60-request IP boundary.

Phase 4 is complete. Production deployment must route storefront
`/api/*` requests to the Rust API on the same public origin, preserving cookies
and `Set-Cookie` headers over HTTPS; the Vite proxy only covers development.

## Phase 5 progress

Cart and checkout preparation are implemented:

- persistent 30-day carts identified by opaque HTTP-only browser tokens stored
  only as SHA-256 hashes;
- server-owned cart lines that accept only variant IDs and quantities, never
  browser-supplied prices;
- add, quantity-update, and remove operations with payload-bound idempotency
  keys that reject accidental key reuse for a different mutation;
- one-currency cart enforcement and reconciliation against current catalog
  prices, publication state, and available inventory on every response;
- explicit price-change, unavailable-product, insufficient-quantity, and
  currency-change issues that prevent a cart from becoming checkout-ready;
- no stock reservation during disposable cart activity;
- guest contact/address creation and updates bound transactionally to the cart,
  closing the remaining Phase 4 checkout-lifecycle gap;
- authenticated delivery capture that reuses the registered customer identity
  while keeping staff and customer sessions separate;
- a responsive storefront cart with quantity controls, removal, delivery
  capture, summaries, reconciliation messages, empty/loading/error states, and
  hydration-safe add-to-cart actions;
- bounded `FOR UPDATE SKIP LOCKED` cleanup for expired carts, with configurable
  1–1000 record batches;
- generated OpenAPI/TypeScript contracts and client methods for the complete
  cart surface;
- isolated PostgreSQL coverage for server pricing, stock/publication
  reconciliation, idempotent retries and conflicts, guest and registered
  delivery ownership, expiration, cleanup, and hashed token storage;
- desktop/mobile browser coverage for empty carts, product addition, delivery
  capture, WCAG A/AA checks, and responsive behavior.

Phase 5 is complete.

## Phase 6 progress

Order creation is implemented:

- immutable product, variant, SKU, price, currency, customer-contact, and
  delivery-address snapshots that remain stable when source records change;
- database-enforced order, payment, and fulfillment states with consistent
  non-negative totals and human-readable sequential order numbers;
- idempotent cart-to-order conversion protected by the cart row and a unique
  cart relationship, so retries return the original order;
- checkout-time catalog, price, currency, delivery, and inventory revalidation;
- atomic inventory reservation during order creation and inventory commitment
  when payment is recorded, using the shared inventory service transaction;
- pending manual-payment records and a development/test-only staff operation
  that confirms the order and records an audited reason;
- append-only order timeline events and immutable order lines;
- capability-protected admin order list and detail APIs using `orders.read`,
  with manual payment protected by `orders.fulfill`;
- OpenAPI and generated TypeScript contracts for storefront and admin order
  workflows;
- an accessible storefront create-order action and confirmation state;
- a responsive admin Orders workspace with customer/delivery snapshots, line
  details, statuses, timeline, and manual-payment control;
- live customer order counts in the existing customer detail workspace;
- isolated PostgreSQL lifecycle coverage for snapshot immutability, retry
  safety, inventory transitions, admin visibility, payment auditing, and
  timeline events;
- desktop/mobile storefront coverage for order confirmation and admin browser
  coverage for order inspection and manual payment.

Phase 6 is complete.

## Phase 7 progress

Stripe payments are implemented:

- a narrow asynchronous payment-provider interface with a Stripe Checkout
  adapter and stable provider idempotency keys;
- validated all-or-nothing Stripe configuration, live keys and HTTPS required
  in production, and a pinned `2026-02-25.clover` API version;
- server-created 35-minute hosted Checkout sessions using authoritative order
  totals, currency, contact email, and internal order/attempt metadata;
- opaque cart-token ownership checks for payment initiation and customer order
  retrieval, with no card data entering KnitPrint;
- raw-body HMAC-SHA256 webhook verification, five-minute replay tolerance,
  duplicate-event suppression, and safe terminal-state handling;
- payment attempts and append-only provider status history exposed in typed
  order contracts and the admin order workspace;
- verified paid events as the only production path that confirms orders and
  commits reserved inventory;
- failed and expired events that cancel unpaid orders and transactionally
  release inventory, while late failure events cannot regress a paid order;
- bounded `FOR UPDATE SKIP LOCKED` abandoned-payment cleanup with a one-hour
  delayed-webhook grace period, immutable payment/order/audit history, and
  repeat-safe scheduling;
- a storefront payment-options contract, Stripe redirect/resume flow, owned
  return-page order loading, and bounded status polling for delayed webhooks;
- preservation of the development-only audited manual-payment lifecycle;
- generated OpenAPI/TypeScript support and PostgreSQL coverage for signed
  success, duplicate delivery, out-of-order failure, expiration, cleanup,
  inventory commitment/release, and manual-payment compatibility.

Phase 7 is complete.

## Phase 8 progress

Fulfillment and transactional notifications are implemented:

- immutable fulfillment and line records supporting partial and complete
  shipments, carrier and tracking references, staff reasons, and actor history;
- paid-and-confirmed eligibility checks, order-row locking, and validation
  against each line's remaining quantity so concurrent operations cannot
  over-fulfill an order;
- payload-bound fulfillment idempotency keys whose exact retries return the
  original result while conflicting reuse is rejected;
- automatic `partially_fulfilled` and `fulfilled` state transitions, with final
  fulfillment completing the order and every shipment appending order timeline
  and audit records;
- a responsive admin paid-order queue, remaining-quantity fulfillment form,
  carrier/tracking capture, shipment history, and notification delivery status;
- transactionally enqueued, deduplicated order-confirmation and
  fulfillment-created notification jobs, keeping commercial state independent
  from email-provider availability;
- a bounded `FOR UPDATE SKIP LOCKED` delivery worker with stale-claim recovery,
  eight-attempt limits, exponential retry delays, and immutable attempt history;
- development-mailbox and AWS SES delivery for confirmation and shipping
  messages, including tracking links when supplied;
- generated OpenAPI/TypeScript contracts and client support for fulfillment
  creation and the expanded order detail;
- PostgreSQL lifecycle coverage for authorization, partial/final fulfillment,
  idempotent replay and conflict, notification failure/retry, durable commercial
  state, and email delivery;
- browser coverage for finding a paid order, creating its shipment, and seeing
  its tracking, timeline, and notification status.

Phase 8 is complete.

## Phase 9 progress

Cancellation and refunds are implemented:

- capability-protected cancellation of unpaid, unfulfilled orders with Stripe
  Checkout expiration when an external session exists;
- atomic release of reserved inventory and explicit cancelled order/payment
  states;
- server-priced partial and full refunds based on immutable order-line
  quantities and the remaining paid balance;
- Stripe Refund API integration using stable provider idempotency keys and the
  payment-intent reference captured from verified Checkout webhooks;
- signed `refund.created`, `refund.updated`, and `refund.failed` webhook
  handling for asynchronous provider outcomes;
- development/test manual refunds without a live provider call;
- explicit per-refund restocking decisions and a committed-to-available
  inventory transition with immutable movement history;
- row-locked eligibility and quantity checks, one in-flight refund per order,
  payload-bound idempotent replay, and conflicting-key rejection;
- immutable cancellation, refund, refund-line, payment-history, audit, and
  order-timeline records with staff reasons and private internal notes;
- server-reported cancellation/refund eligibility and remaining refundable
  balance in the order contract;
- typed OpenAPI/TypeScript client methods and responsive admin cancellation,
  refund, restocking, and refund-history controls;
- PostgreSQL lifecycle coverage for authorization, cancellation, partial then
  full manual refunds, Stripe adapter refunds, exact replay, conflict handling,
  auditing, state transitions, and inventory restocking;
- a real admin/storefront browser journey through payment, fulfillment, full
  refund, refund history, private notes, restocking, and mobile overflow.

Phase 9 is complete.

## Phase 10 progress

Discounts are implemented:

- capability-protected administration of fixed-amount and percentage codes,
  with normalized unique codes, active date windows, currency, minimum order,
  global usage limits, optional per-customer limits, and enable/disable status;
- validated integer minor-unit and basis-point inputs, bounded date and usage
  rules, staff reasons, actor attribution, and immutable audit history for
  creation and status changes;
- non-mutating cart evaluation with apply/remove operations, explicit
  unavailable-code feedback, and server-derived subtotal, discount, and total;
- checkout-time recalculation under a discount row lock, with usage counting in
  the same transaction as order creation so concurrent checkout cannot exceed
  a global limit;
- immutable order discount snapshots containing the code, rule configuration,
  subtotal, discount amount, and evaluated timestamp, preserving historical
  orders after a code is disabled or changed;
- customer-visible cart and order-confirmation discount totals, plus a
  responsive admin Discounts workspace for code creation, inspection, and
  status management;
- generated OpenAPI and TypeScript schemas and client operations for the cart
  and admin discount APIs;
- PostgreSQL lifecycle coverage for fixed/percentage pricing, snapshot
  stability, authorization, usage recording, and the exact concurrent global
  limit boundary;
- a real admin/storefront browser journey that creates a code, applies its
  lowercase form, verifies the server-priced total, and sees its snapshot on
  the resulting order.

Phase 10 is complete.

## Phase 11 progress

Commercial settings, shipping, and tax pricing are implemented:

- a singleton non-secret store identity with store name, support email,
  currency, and an explicit destination-tax enablement switch;
- capability-protected transactional settings replacement with normalized
  country codes, non-overlapping active destination groups, bounded flat rates
  and tax basis points, required staff reasons, immutable settings history, and
  global audit records;
- ordered shipping zones with an explicit-country-first worldwide fallback,
  multiple active flat-rate methods per zone, and customer method selection;
- a jurisdiction-neutral exclusive tax engine that is disabled by default and
  requires a matching configured destination rule when enabled, without
  seeding or claiming any production statutory rate;
- deterministic server pricing in the planned order: base merchandise,
  discount, shipping, destination tax, and final total;
- checkout locking of the settings singleton and selected commercial rows so a
  concurrent settings change cannot produce a mixed rule set;
- immutable order shipping and tax snapshots containing source identifiers,
  display names, destination, currency, taxable base, rate, behavior, and
  amounts, including migration backfill for pre-Phase-11 orders;
- storefront shipping choices and complete subtotal/discount/shipping/tax/total
  presentation in the cart and order confirmation;
- a responsive admin Settings workspace for store identity, zones, methods,
  tax rules, integration configuration status, and recent actor-attributed
  history, without accepting or exposing runtime secrets;
- generated OpenAPI and TypeScript contracts for settings and cart shipping
  selection;
- PostgreSQL coverage proving calculation, authorization, audit history, and
  snapshot stability after settings replacement;
- a real admin/storefront browser journey that configures standard and express
  shipping plus a test-only destination rate, selects the express method, and
  verifies the resulting order totals and snapshots.

Phase 11 is complete.

## Phase 12 progress

The operational dashboard is implemented:

- a private, capability-aware aggregate endpoint that exposes order sections
  only to staff with `orders.read` and inventory sections only to staff with
  `inventory.adjust`;
- explicit server-owned metric definitions and UTC boundaries, including all
  order counts, current-UTC-day orders, captured gross revenue, succeeded
  refunds, net revenue, paid fulfillment work, failed payments, and low-stock
  variants;
- revenue constrained to the configured store currency while order counts are
  explicitly documented as spanning currencies and states;
- bounded eight-record queues for paid orders awaiting fulfillment, recent
  orders, low-stock variants, current failed payments, and recent refunds;
- direct `#orders/{id}` and `#inventory/{variant_id}` links that open the exact
  actionable record instead of leaving staff to search again;
- dashboard refetch-on-open behavior so operational state changes are visible
  immediately rather than waiting for the general reference-data cache;
- targeted partial indexes for fulfillment work, failed payments, and recent
  cross-order refund activity;
- responsive operational metric cards, queue panels, empty states, and an
  inspectable definition panel showing generation time and timezone;
- generated OpenAPI and TypeScript support for every dashboard metric and
  queue item;
- PostgreSQL lifecycle coverage for authentication, capability filtering, UTC
  periods, paid queue transitions, low stock, failed payments, gross/refund/net
  totals, and bounded record identifiers;
- browser and accessibility coverage proving dashboard freshness, responsive
  behavior, and direct navigation from fulfillment, inventory, and refund
  queues to their source records.

Phase 12 is complete. The section-11 security and operational hardening
milestone is addressed in Phase 13 below.

## Phase 13 progress

The dedicated security and operational hardening milestone is implemented:

- production requires an explicit exact-HTTPS `WEB_ORIGINS` allowlist;
- unsafe cross-origin browser mutations are rejected using `Origin` and Fetch
  Metadata, credentialed CORS is allowlisted, request bodies are capped at 1
  MiB, and defensive response headers are applied centrally;
- staff cookies remain strict, customer cookies remain intentionally lax for
  ordinary storefront navigation, and all production cookies are secure,
  HTTP-only, opaque, hashed server-side, expiring, revocable, and cleaned;
- registration, verification-email, and password-reset requests now have
  concurrency-safe persistent account/IP/global abuse limits with hashed keys,
  complementing the existing exact login limiter;
- quarantined media must pass a production-required ClamAV-compatible INSTREAM
  scan, declared MIME/signature matching, dimension/pixel/allocation bounds, and
  normalization before it can become ready; scanner failure fails closed and an
  infected result creates an immutable audit event;
- production S3 supports the standard AWS credential chain for workload roles,
  with prefix-scoped IAM, TLS-only bucket, and narrow upload-CORS templates;
- production API startup no longer runs migrations and uses bounded statement,
  lock, and idle-transaction timeouts; separate migration/runtime/reporting
  PostgreSQL grants are provided;
- production request tracing is structured JSON with request IDs/status/latency,
  panics are converted to non-disclosing JSON errors, and a machine-readable
  operational backlog check exits nonzero for durable work requiring attention;
- a safe explicit-source/destination backup/restore verification script checks
  successful migrations, table parity, and core commercial record counts after
  a custom-format logical restore;
- CI adds RustSec, filesystem/secret/misconfiguration scanning, a non-root API
  container build and image scan, a PostgreSQL backup/restore drill, operational
  backlog verification, and a bounded k6 public API load gate;
- a production runbook maps every section-11 control to evidence and documents
  the launch gate, AWS/PostgreSQL obligations, alerts, secret rotation, recovery
  drill, retention workers, and first incident actions.

Phase 13 is complete at the application/repository level. Production launch is
still gated on executing and recording the runbook checks in the real AWS,
PostgreSQL, Stripe, SES, malware-scanner, monitoring, and backup environments;
those external controls cannot be truthfully verified from this repository.

The complete Rust workspace suite (including PostgreSQL authorization,
ownership, concurrency, idempotency, retention, webhook, media, and abuse-limit
coverage), Clippy with warnings denied, API contract check, TypeScript checks,
production builds, the 22-test storefront/account browser suite, and all four
admin browser journeys passed on 2026-08-15. The live npm production audit
identified two high-severity transitive advisories; the lockfile was advanced
to `js-yaml` 4.3.1 and `nanoid` 3.3.18, after which the audit reported zero
vulnerabilities and the frontend checks/builds passed again. The pinned-base,
non-root API container built successfully and its configured user was verified
as `10001:10001`. The operational backlog check reported healthy, and live
local HTTP checks confirmed security headers and hostile-origin rejection. CI
will run the RustSec/Trivy scans, backup/restore drill, and k6 gate in its clean
tooling environment. No request was sent to live Stripe, SES, AWS S3, or a
production malware scanner because their production credentials/services are
not committed or available locally.

## Environment notes

- No deployment has been performed.
- PostgreSQL and MinIO were verified with Docker Compose and left running for
  the next feature slice.
- Production now requires `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, and an
  HTTPS `STOREFRONT_BASE_URL`; configure the documented webhook event set and
  schedule `npm run admin:cleanup-payments` before deployment. Schedule
  `npm run admin:deliver-notifications` frequently enough for the required
  delivery latency and alert on terminally failed jobs.
- Preserve the user's untracked `commands.md` during future work.
- Tax calculation remains disabled in the seed configuration. Production tax
  rules must be confirmed for every served jurisdiction before enablement; the
  implementation deliberately does not encode legal or statutory assumptions.
- Generated public logo assets are reproducible with `npm run assets:brand`;
  the source at `images/logo.png` remains authoritative and untouched.
