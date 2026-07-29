# Store Implementation Plan

Last updated: 2026-07-29

## 1. Goal

Build a deliberately simple ecommerce store with:

- a public customer storefront;
- a secure operational admin area;
- a warm, craft-led visual identity based on the supplied KnitPrint logo;
- a Rust backend;
- low and predictable server resource usage;
- features delivered as independent vertical slices;
- clear extension points for search, media processing, payments, and other
  infrastructure without prematurely adopting complex systems.

The previous Medusa implementation is not the basis for this plan. It remains in
the repository only as historical work until a separate cleanup is approved.

### 1.1 Brand and storefront direction

The store name is **KnitPrint**. The supplied source logo is
`images/logo.png`. Its yarn, knitted lettering, printed cube, and extruder
combine handmade craft with digital fabrication; the storefront should feel
warm, tactile, and contemporary rather than generically technical.

Use this initial color system, refined from the logo:

```text
Canvas / warm ivory       #F7F3EF
Surface / soft white      #FFFDFC
Yarn / warm beige         #B99A79
Yarn dark / brown         #765C47
Knit / muted mauve        #8A6877
Knit dark / plum          #5F4652
Ink / charcoal            #2E2E2D
Muted ink                 #6E6864
Border                    #DED4CC
```

The precise values may be tuned during implementation after checking them in
the browser, but the visual relationship should remain: ivory backgrounds,
charcoal text, mauve brand actions, and beige highlights. Avoid using the beige
for small text and verify every text/background pairing against WCAG AA.

Typography should pair a friendly display face for short headings with a highly
readable sans serif for navigation, product information, forms, and prices.
The implementation should prefer self-hosted or system fonts and avoid making a
third-party font service a runtime dependency.

Logo preparation should create optimized derivatives from the supplied image:

- a tightly cropped full wordmark for the storefront header and footer;
- a compact mark for small screens and the favicon;
- WebP/AVIF derivatives where browser support and quality are appropriate;
- dimensions that reserve layout space and avoid cumulative layout shift.

Do not distort, recolor, or place the logo over visually noisy imagery. Preserve
the original in `images/logo.png`; generated web assets should live with the
frontend's static assets.

The initial storefront experience should be product-led and restrained:

- a compact header with logo, Shop, About, search, account, and cart;
- a homepage hero that explains the craft proposition in one sentence;
- featured collections or categories;
- a clean product grid with generous image space;
- an editorial brand/story band connecting knitting and 3D printing;
- a concise reassurance band for shipping, returns, and secure checkout;
- a useful footer with customer-help and store-information links.

Rounded corners, soft borders, shallow shadows, and occasional yarn-like curves
may echo the logo. Texture should be used sparingly so products remain the
visual focus. Motion must be subtle and respect `prefers-reduced-motion`.

## 2. Guiding principles

### 2.1 Build a modular monolith

Start with one Rust application organized into feature modules. Do not introduce
microservices or multiple Rust crates until a real boundary or deployment need
justifies them.

Each feature should own:

- its database migrations and queries;
- domain types and business rules;
- application services and transaction boundaries;
- HTTP handlers and OpenAPI definitions;
- admin and storefront UI where relevant;
- authorization and audit behavior;
- tests and fixtures.

Features should communicate through small public service interfaces rather than
modifying one another's tables directly.

### 2.2 Deliver vertical slices

Every increment should produce a coherent, usable capability. A feature is not
complete when only its schema, API, or UI exists.

Each slice should include:

1. PostgreSQL migration;
2. Rust domain and application logic;
3. explicit SQLx queries;
4. Axum endpoints;
5. OpenAPI contract;
6. generated TypeScript types/client;
7. TanStack Query integration;
8. admin and/or storefront UI;
9. authorization and audit rules;
10. automated tests and documentation.

### 2.3 Keep authoritative data on the server

- PostgreSQL is authoritative for commercial and operational data.
- S3 is authoritative for uploaded file content.
- Prices, discounts, stock, and order eligibility are always recalculated or
  validated by the Rust backend.
- Search indexes, browser databases, caches, and generated media are rebuildable
  projections.

### 2.4 Prefer explicit behavior

- Use handwritten SQL and visible transaction boundaries.
- Model business state changes as named operations.
- Use database constraints to protect important invariants.
- Make external and retryable operations idempotent.
- Snapshot historical commercial data on orders.
- Avoid abstractions and infrastructure that are not yet needed.

### 2.5 Separate rendering responsibilities

The two web surfaces have intentionally different rendering models:

- The admin is a private, fully client-rendered application. It talks directly
  to the Rust API and does not require SSR.
- The storefront is an SSR application. Public pages must return meaningful
  HTML for initial performance, accessibility, search engines, and link
  previews.
- Storefront components may become interactive after hydration. Complex
  client-side experiences are expected rather than treated as exceptions.
- The Rust API remains authoritative. Storefront SSR may read from the same
  public API but must not reimplement commercial business rules.

Server rendering and browser hydration must produce consistent data. Any
customer action that changes commercial state still goes through the Rust API.

## 3. Proposed technology stack

### Frontend

- A TanStack-based React SSR application for the public storefront
- A React/Vite SPA for the private admin
- TanStack Router or TanStack Start routing as appropriate to each surface
- TanStack Query
- TanStack Form
- TanStack Table
- Shared generated TypeScript API types/client from OpenAPI
- Shared UI and domain-presentation packages only where they provide real value

The storefront must support SSR and selective client hydration. Product,
category, search, and other public discovery pages should render useful HTML on
the server. Interactive features such as cart controls, rich product selectors,
live search, account tools, and checkout flows can be client components backed
by TanStack Query and Form.

Avoid hydrating an entire page merely because one section is interactive.
Define server-rendered page boundaries and focused interactive components, and
keep browser-only libraries out of the server-rendering path.

### Backend

- Rust
- Axum
- Tokio
- Serde
- `utoipa` or an equivalent OpenAPI generator
- Structured tracing and error reporting

The Rust service exposes `/api/*` and may serve the compiled admin SPA. A
separate storefront rendering process handles React SSR. In production, a
reverse proxy or edge router should present them through one origin:

```text
/api/*    -> Rust/Axum API
/admin/*  -> static admin SPA
/*        -> storefront SSR
```

Using one public origin simplifies cookies, CSRF policy, CORS, and local
development. The SSR process should be a thin presentation layer, not a second
business backend.

### Database

- PostgreSQL
- Handwritten SQL migrations
- SQLx for pooling, transactions, query execution, and compile-time checked
  queries
- SQLx offline metadata for CI and production builds where appropriate

No ORM or Drizzle is planned.

### External infrastructure

- AWS S3 for all persistent image and file content
- CloudFront for stable public media delivery
- Stripe for payments
- A transactional email provider
- MinIO or another S3-compatible service for local development

## 4. Initial repository shape

```text
store/
├── frontend/
│   ├── src/
│   │   ├── api/generated/
│   │   ├── features/
│   │   └── routes/
│   │       ├── admin/
│   │       └── shop/
│   └── package.json
├── backend/
│   ├── src/
│   │   ├── features/
│   │   │   ├── auth/
│   │   │   ├── catalog/
│   │   │   ├── inventory/
│   │   │   ├── customers/
│   │   │   ├── carts/
│   │   │   ├── orders/
│   │   │   ├── payments/
│   │   │   ├── fulfillment/
│   │   │   ├── discounts/
│   │   │   ├── media/
│   │   │   └── settings/
│   │   ├── infrastructure/
│   │   └── main.rs
│   └── Cargo.toml
├── migrations/
└── compose.yaml
```

Within a feature, prefer a structure similar to:

```text
catalog/
├── domain.rs
├── service.rs
├── repository.rs
├── http.rs
└── tests.rs
```

This structure is a starting convention rather than a requirement to create
empty files for every possible layer.

## 5. Admin scope

The admin is an operations tool, not initially a general-purpose CMS or
business-intelligence platform.

Its eventual primary navigation should be:

```text
Dashboard
Orders
Products
Customers
Discounts
Settings
```

Inventory is initially managed within products, and fulfillment is managed
inside an order.

The admin must eventually support:

- an actionable operational dashboard;
- products, variants, pricing, images, and publication status;
- inventory levels, adjustments, and movement history;
- searchable orders and complete order details;
- payment, fulfillment, cancellation, and refund operations;
- lightweight customer records and order history;
- basic discount codes;
- store, shipping, tax, and integration settings;
- staff authentication, permissions, and audit history.

## 6. Incremental delivery plan

### Phase 0: Project foundation

Deliver a runnable but intentionally empty application.

Build:

- TanStack Start SSR storefront, React/Vite admin, and Rust/Axum backend;
- PostgreSQL and SQLx;
- environment validation;
- forward SQL migrations and seed commands;
- admin and storefront layouts;
- the KnitPrint design tokens, responsive branded header, footer, and storefront
  shell;
- optimized full and compact logo assets derived from `images/logo.png`;
- representative responsive homepage sections using seeded placeholder content;
- health and database readiness endpoints;
- structured logging and consistent error responses;
- unit, integration, and browser test foundations;
- CI checks for formatting, linting, types, tests, SQLx metadata, and production
  builds;
- shared conventions for IDs, timestamps, currencies, money, pagination, and
  idempotency keys;
- OpenAPI generation and TypeScript client generation;
- development S3-compatible storage;
- initial `ProductSearch` and `MediaStorage` interfaces.

Acceptance:

- A fresh checkout can be installed, migrated, seeded, tested, built, and
  started using documented commands.
- `/` renders the storefront shell.
- `/admin` renders the admin shell.
- Health checks verify both the application and PostgreSQL.
- The storefront shell works at mobile, tablet, and desktop widths without
  horizontal overflow or layout shift from the logo.
- Brand colors are expressed as reusable semantic tokens rather than scattered
  literal values.
- Keyboard focus, text contrast, reduced motion, and landmark structure pass
  the initial accessibility checks.

### Phase 1: Staff authentication and authorization

Build:

- staff users;
- secure password authentication;
- secure HTTP-only cookie sessions stored or validated by the backend;
- `owner` and `staff` roles;
- capability-based authorization;
- login, logout, and profile UI;
- initial owner creation command;
- staff creation and disabling;
- immutable audit-log foundation.

Initial capabilities should include:

```text
catalog.read
catalog.write
inventory.adjust
orders.read
orders.fulfill
orders.refund
customers.read
media.upload
media.review
staff.manage
settings.manage
```

Acceptance:

- Anonymous users cannot access admin data or operations.
- An owner can sign in and manage staff.
- Every server operation checks authorization independently of the UI.
- Sensitive operations can record actor, action, entity, reason, and time.

### Phase 2: Catalog, admin media uploads, and product search

Build:

- products and variants;
- draft, active, and archived states;
- title, description, slug, SKU, price, currency, and search keywords;
- product and variant image relationships;
- image ordering and alt text;
- admin product list, editor, and preview;
- public product listing and product page;
- admin and storefront search using PostgreSQL;
- presigned S3 uploads for authorized admin images;
- upload completion verification;
- basic normalized image variants for thumbnail, card, and detail views;
- abandoned-upload cleanup;
- stable CloudFront delivery URLs for public images.

Acceptance:

- Staff can create a draft, upload images, add variants, and publish a product.
- Published products appear in the storefront and search results.
- Draft and archived products are never publicly purchasable.
- Prices use integer minor units and an explicit currency.
- SKU and slug uniqueness is enforced by PostgreSQL.
- Permanent signed URLs are not stored in the database.

This is the first complete business feature and should remain usable without
inventory, carts, or payments.

### Phase 3: Inventory

Build:

- inventory items linked to variants;
- available, reserved, and committed quantities;
- manual adjustments with required reasons;
- immutable inventory movement history;
- low-stock thresholds and admin indicators;
- transactional protection against overselling;
- an inventory interface similar to:

```text
get_availability
reserve
release
commit
adjust
```

Acceptance:

- Staff can adjust stock and inspect its history.
- Concurrent operations cannot oversell a variant.
- Catalog code obtains availability through the inventory service rather than
  modifying inventory records.
- The first version assumes one stock location.

Phases 0 through 3 form the first milestone: a secure admin can publish real
products, manage images and stock, and expose a searchable read-only storefront.

### Phase 4: Customers and addresses

Build:

- guest customer records;
- optional customer accounts;
- customer contact details and addresses;
- admin customer list and detail screen;
- customer order-history placeholder;
- ownership-based access controls;
- privacy-aware data access and retention rules.

Acceptance:

- A guest or registered customer can provide contact and delivery information.
- Staff with permission can find and inspect a customer.
- Staff authentication remains separate from customer authentication.
- Guest checkout remains possible even if customer accounts are deferred.

### Phase 5: Cart and checkout preparation

Build:

- persistent carts and line items;
- add, update, and remove operations;
- server-controlled price calculation;
- availability reconciliation;
- shipping address capture;
- cart summary;
- cart expiration;
- storefront cart and checkout preparation UI.

Acceptance:

- The server never trusts prices supplied by the browser.
- Unavailable, unpublished, or repriced products are reconciled before checkout.
- Cart operations are safe under repeated requests.
- A cart remains disposable state and is not treated as an order.

### Phase 6: Order creation

Build:

- orders and immutable order-line snapshots;
- address, product, SKU, price, discount, shipping, and tax snapshots;
- order numbers and totals;
- explicit order, payment, and fulfillment states;
- admin order list and detail view;
- development-only manual payment method;
- order timeline;
- idempotent cart-to-order conversion.

Initial state model:

```text
Order:      pending -> confirmed -> completed | cancelled
Payment:    pending -> authorized/paid -> refunded | failed
Fulfillment: unfulfilled -> fulfilled
```

Acceptance:

- A valid cart converts into exactly one order.
- Product edits do not change historical order data.
- Repeated checkout requests cannot duplicate orders.
- Staff can inspect every relevant order detail.

### Phase 7: Stripe payments

Build:

- a narrow payment-provider interface;
- Stripe checkout or payment-intent creation;
- signed webhook validation;
- idempotent and out-of-order webhook processing;
- payment attempts and status history;
- failed and abandoned payment behavior;
- inventory reservation, commitment, and release tied to documented state
  transitions.

Provider operations should resemble:

```text
create_payment
capture_payment
cancel_payment
refund_payment
handle_webhook
```

Acceptance:

- Only a verified server-side payment event can mark an order paid.
- Repeated webhooks and requests are safe.
- Card data never enters the application or database.
- Failed payment cannot create a fulfillable order.

### Phase 8: Fulfillment and notifications

Build:

- complete or line-level fulfillment;
- carrier and tracking references;
- fulfillment history;
- confirmation and fulfillment emails;
- admin queue for orders needing fulfillment;
- retryable background notification work.

Acceptance:

- Staff can find and fulfill paid orders.
- Customers receive confirmation and shipping information.
- Email failure does not roll back payment or fulfillment.
- Repeated actions do not duplicate fulfillment or notifications.

### Phase 9: Cancellation and refunds

Build:

- cancellation before fulfillment;
- complete and partial refunds;
- Stripe refund integration;
- restocking decisions;
- reasons and internal notes;
- audit records and order timeline events.

Acceptance:

- Eligibility is visible before an action is attempted.
- Refund requests are idempotent.
- Payment, order, and inventory states remain consistent.
- Every sensitive action records its staff actor and reason.

This phase should be complete before the store is considered operationally ready
for general use.

### Phase 10: Discounts

Build:

- fixed and percentage discount codes;
- active dates;
- minimum order amount;
- global usage limits;
- optional per-customer limits;
- discount snapshots on orders;
- deterministic pricing pipeline:

```text
base prices -> discounts -> shipping -> tax -> final total
```

Acceptance:

- Discount evaluation does not mutate the cart.
- Calculation is deterministic and controlled by the server.
- Editing a discount never changes an existing order.
- Concurrent checkout cannot exceed a global usage limit.

### Phase 11: Shipping, taxes, and settings

Build:

- store identity and currency;
- shipping zones and methods;
- initially simple flat-rate shipping;
- tax behavior appropriate to the operating jurisdictions;
- email sender configuration status;
- integration health indicators;
- settings audit history.

Acceptance:

- Staff can manage ordinary commercial settings without deployment.
- Secrets remain in environment variables or a secret manager.
- Orders snapshot all commercial rules and amounts.
- Settings changes do not modify historical orders.

Tax requirements must be confirmed for the countries served before implementing
production tax behavior.

### Phase 12: Operational dashboard

Build only after the underlying operational data exists:

- paid orders awaiting fulfillment;
- recent orders;
- low-stock variants;
- failed payments;
- recent refunds;
- basic order and revenue totals.

Acceptance:

- Every dashboard item links to actionable source records.
- Metrics have documented definitions and timezone handling.
- No business rules live solely in dashboard queries or UI.

## 7. Search strategy

### 7.1 Initial PostgreSQL search

Initial real-time, search-as-you-type behavior should use:

- PostgreSQL full-text search for titles and descriptions;
- `pg_trgm` for partial matching and basic typo tolerance;
- indexes for categories, prices, status, availability, and common ordering;
- a dedicated product search projection where useful;
- roughly 100-200 ms request debouncing;
- cancellation of superseded requests;
- cursor-based result pagination.

The HTTP contract must not expose its storage implementation:

```text
GET /api/search/products?q=shirt&category=men&limit=20
```

Catalog handlers should depend on a `ProductSearch` interface. The first adapter
is `PostgresProductSearch`.

### 7.2 Dedicated server-side search

If PostgreSQL search becomes a measured bottleneck, evaluate:

1. query and index optimization;
2. caching or a PostgreSQL read replica where appropriate;
3. Typesense, Meilisearch, or OpenSearch behind the same interface.

A dedicated server-side index is normally the scaling response to a large
catalog or high search traffic.

### 7.3 Browser-local search

IndexedDB may later store a deliberately limited public search projection for:

- near-zero-latency repeated searches;
- offline catalog browsing;
- reduced API traffic;
- selected admin lookup workflows over appropriately authorized data.

Do not synchronize private admin or customer data wholesale.

Synchronization should use a revisioned change feed rather than repeatedly
downloading the complete catalog:

```text
GET /api/catalog/sync?afterRevision=1842
```

The change feed must include tombstones for unpublished or deleted records.
Local results are discovery data only; the backend must revalidate prices and
stock at checkout.

SQLite with WebAssembly and FTS5 may be evaluated if local queries outgrow
IndexedDB. This is a future adapter, not an initial commitment, because it adds
WASM loading, persistence, worker communication, schema upgrades, and
synchronization complexity.

Search alternatives should be selected only after measuring catalog size,
server latency, request volume, index size, synchronization cost, data freshness,
and mobile browser performance.

## 8. Media and file architecture

### 8.1 Asset model

One media subsystem should support different purposes and trust levels:

- admin product images;
- customer-submitted images;
- customer documents;
- private internal attachments;
- public branding and site assets.

Store object keys and metadata in PostgreSQL, while storing file content in S3.
An asset record should be able to represent:

```text
id
owner_type
owner_id
purpose
visibility
original_object_key
processed_object_key
original_filename
declared_media_type
detected_media_type
byte_size
checksum
validation_status
rejection_reason
created_by
created_at
```

Product relationships should be separate so assets can have ordering, alt text,
and controlled reuse.

### 8.2 Upload flow

Use direct, presigned uploads:

1. The browser requests authorization for a particular file and purpose.
2. Rust verifies identity, permission, limits, and declared metadata.
3. Rust creates a pending media record and server-generated object key.
4. Rust returns a short-lived presigned S3 PUT or POST operation.
5. The browser uploads directly to a private quarantine location.
6. The browser calls a completion endpoint.
7. Rust verifies object existence, size, and checksum.
8. A worker scans, validates, and processes the object.
9. Accepted derivatives move to their final public or private location.
10. Rejected or abandoned objects are removed through cleanup and lifecycle
    policies.

Presigned POST should be considered when S3-enforced content type and length
conditions are useful.

### 8.3 Asset lifecycle

```text
pending_upload
    -> uploaded
    -> quarantined
    -> scanning
    -> accepted
    -> processing
    -> ready

scanning -> rejected
processing -> rejected
```

Customer-submitted content must not be made public, served to another user, or
consumed by another system until it is ready.

### 8.4 Validation

Validation should cover:

- user permission and record ownership;
- allowed purpose;
- per-purpose file count and maximum size;
- checksum;
- file signature or magic bytes;
- detected rather than declared MIME type;
- successful format decoding;
- image dimensions and total pixel count;
- malware scanning;
- filename normalization;
- format-specific safety limits.

Accepted images should normally be decoded and re-encoded into formats we
control. This strips most unwanted metadata and prevents public delivery of the
original untrusted bytes.

SVG should initially be rejected unless a conservative sanitizer and a specific
requirement justify it. Executables, HTML, scripts, archives, and macro-enabled
documents should be rejected unless explicitly required later.

### 8.5 Storage and delivery boundaries

Use separate buckets or clearly isolated prefixes for:

```text
uploads-quarantine/
media-public/
media-private/
```

Rules:

- Quarantine is never publicly readable.
- Public content is delivered through stable CloudFront URLs.
- Private content requires authorization and a short-lived download URL.
- S3 public access should remain blocked.
- CloudFront should access approved objects using restricted origin access.
- Object keys are server-generated and do not confer authorization.
- Original filenames are metadata only.
- Customer access is determined from database ownership.
- AWS credentials never enter the browser.
- Lifecycle policies remove abandoned and rejected objects.
- Permanent deletion is performed by controlled cleanup work.

Public product images should not use expiring S3 download URLs. Signed download
URLs are reserved for private assets.

### 8.6 AWS permissions

Use narrowly scoped identities:

- the Rust API can authorize constrained uploads;
- the media worker can read quarantine and write processed objects;
- CloudFront can read approved public content;
- a browser receives permission for one short-lived object operation;
- no component receives bucket-wide administrative access without need.

## 9. Database integrity, optimization, and permissions

### 9.1 Schema integrity

Use PostgreSQL constraints wherever practical:

- foreign keys;
- unique constraints;
- `NOT NULL`;
- check constraints;
- explicit referential deletion behavior;
- non-negative inventory invariants;
- valid currency and amount combinations;
- idempotency-key uniqueness;
- unique external webhook event IDs;
- version columns where concurrent editing needs optimistic control.

Rust validates input and coordinates workflows. PostgreSQL remains the final
line of defense for stored invariants.

### 9.2 Indexing

Every migration should document the queries introduced by its feature and add
only the indexes that support demonstrated access patterns.

Likely indexes include:

- unique product slug and SKU;
- active products ordered by publication time;
- full-text and trigram product search;
- orders by customer and creation time;
- paid, unfulfilled orders;
- inventory movements by variant and time;
- customers by normalized email;
- media by owner and purpose;
- pending uploads by expiration;
- discount codes and validity;
- external event identifiers and idempotency keys.

Use partial indexes for operational queues where appropriate, for example paid
orders awaiting fulfillment. Avoid speculative indexes because they consume
storage and increase write cost.

### 9.3 Query discipline

Require:

- explicit column lists rather than `SELECT *`;
- pagination for every potentially unbounded collection;
- cursor pagination for large or frequently changing collections;
- clear transactions around multi-record workflows;
- no accidental N+1 access patterns;
- bounded connection pools;
- query timeouts;
- cancellation of abandoned requests where possible;
- slow-query logging;
- measurement using `EXPLAIN (ANALYZE, BUFFERS)`;
- performance tests with production-like data volumes;
- monitoring of connection saturation, vacuum, table growth, and query latency.

### 9.4 PostgreSQL roles

The application must not run as a PostgreSQL superuser or schema owner.

Use separate roles for:

- migration execution;
- runtime application access;
- read-only operational or reporting access;
- backups and monitoring.

The runtime role must not be able to alter schemas, create roles, or bypass
ordinary controls.

Row-level security may later provide defense in depth for customer-owned records
or private media. Adopt it only after evaluating its connection-context and
policy complexity; application-level ownership checks remain mandatory.

## 10. Cross-feature integration rules

- One PostgreSQL database, with feature-owned tables.
- Public service interfaces for cross-feature operations.
- Database transactions for immediate consistency.
- Post-commit events or durable jobs for non-critical side effects.
- Immutable snapshots for historical order information.
- Idempotency at checkout, webhook, refund, fulfillment, upload-completion, and
  job boundaries.
- Explicit state-transition functions instead of arbitrary status updates.
- One initial currency, stock location, payment provider, and shipping model.
- Stable API contracts hide whether search or media uses a particular adapter.
- No browser-side projection is trusted for commercial decisions.

Background work must be durable and retryable before it is used for operations
whose loss would matter. Email failure must not roll back payments or
fulfillment.

## 11. Security and operational hardening milestone

Before accepting real payments or general customer uploads, perform a dedicated
hardening pass covering:

- authorization coverage;
- ownership checks;
- session and cookie security;
- CSRF and cross-origin behavior;
- rate limits and abuse controls;
- file quarantine and validation;
- malware scanner failure behavior;
- AWS IAM and bucket policies;
- database roles and grants;
- secret storage and rotation;
- webhook signature validation and replay safety;
- audit completeness;
- backup and restore testing;
- dependency and container scanning;
- slow-query and load testing;
- retention and deletion behavior;
- observability and operational alerts.

This milestone verifies the controls built incrementally; it is not permission
to postpone basic security until the end.

## 12. Definition of done for every increment

Each feature must answer:

- What data does it own?
- Who may read and change that data?
- Which database constraints protect it?
- What are its state transitions?
- What happens under concurrent requests?
- Which queries are expected and which indexes support them?
- Are all collections bounded and paginated?
- Which actions require an audit record and reason?
- Does it accept files, and under which validation policy?
- What is public, private, or quarantined?
- What are its retention and deletion rules?
- How are retries and duplicate requests handled?
- How is the feature tested, monitored, backed up, and restored?

The delivery checklist is:

- forward migration reviewed;
- destructive schema evolution staged safely;
- seed or fixture data added;
- domain tests;
- authorization and ownership tests;
- server/API integration tests;
- frontend tests appropriate to the risk;
- concurrency and idempotency tests;
- audit behavior verified;
- query plan and indexes reviewed;
- OpenAPI and generated client updated;
- documentation updated;
- production build verified;
- no unfinished dependency on the next increment.

## 13. Explicitly deferred decisions

The following decisions should remain open until requirements or measurements
justify them:

- precise storefront SSR deployment and optional product-page prerendering;
- customer accounts versus guest-only checkout at first;
- PostgreSQL search versus a dedicated search service;
- IndexedDB versus SQLite WASM for optional local search;
- precise image-processing implementation;
- exact malware scanning service;
- transactional email provider;
- Stripe Checkout versus custom Payment Intents UI;
- PostgreSQL row-level security;
- multi-location inventory;
- multiple currencies;
- background-job technology;
- deployment provider and topology.

These are planned extension points, not missing requirements for the first
milestone.
