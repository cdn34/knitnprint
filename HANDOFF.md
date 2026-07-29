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
- `GET /api/health`;
- database-aware `GET /api/ready`;
- consistent JSON 404 and readiness errors;
- graceful shutdown and bounded PostgreSQL pooling.

## Verified

The following passed on 2026-07-29:

```bash
npm run typecheck
npm run build
cargo fmt --check
cargo test --workspace
```

Runtime smoke checks also verified:

- `/` returns meaningful server-rendered HTML and KnitPrint metadata;
- `/api/health` returns `200` and a JSON health payload;
- `/api/ready` returns a structured `503` when `DATABASE_URL` is absent.

## Local development

Requirements:

- Node.js 24;
- Rust stable;
- Docker for the local PostgreSQL and MinIO services.

Setup and start commands are documented in `README.md`.

## Next implementation slice

Continue Phase 0 with:

1. forward SQL migration and seed command foundations;
2. OpenAPI generation and TypeScript API client generation;
3. automated API handler tests and frontend browser/accessibility tests;
4. environment validation shared by startup commands;
5. CI checks;
6. locally optimized logo derivatives instead of the current source-image
   copies;
7. running PostgreSQL/MinIO integration verification when Docker is available.

After Phase 0 acceptance is complete, start Phase 1 staff authentication and
authorization.

## Environment notes

- No deployment has been performed.
- PostgreSQL and MinIO were not started during the current implementation pass.
- The root `.git` path in this supplied workspace is not a functional Git
  repository, so no Git status or commit verification is available.
- The copied public logo files intentionally preserve the source for now; the
  source at `images/logo.png` remains authoritative.
