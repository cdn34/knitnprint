#!/bin/sh
set -eu

: "${DATABASE_URL:?Set DATABASE_URL to a disposable PostgreSQL 17 test database}"

# Match CI's shared extension setup before tests create their isolated schemas.
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c 'CREATE EXTENSION IF NOT EXISTS citext WITH SCHEMA public'
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run typecheck
npm run build
node --test apps/admin/tests/*.test.mjs apps/storefront/tests/*.test.mjs
npm run api:check
