#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL is required}"
: "${RESTORE_DATABASE_URL:?RESTORE_DATABASE_URL must point to an empty disposable database}"

if [[ "${DATABASE_URL}" == "${RESTORE_DATABASE_URL}" ]]; then
  echo "source and restore databases must be different" >&2
  exit 2
fi

backup_file="$(mktemp /tmp/knitprint-backup-XXXXXX.dump)"
trap 'rm -f "${backup_file}"' EXIT

pg_dump --format=custom --no-owner --no-acl --file="${backup_file}" "${DATABASE_URL}"
pg_restore --exit-on-error --no-owner --no-acl --dbname="${RESTORE_DATABASE_URL}" "${backup_file}"

source_migrations="$(psql "${DATABASE_URL}" -Atc 'SELECT count(*) FROM _sqlx_migrations WHERE success')"
restored_migrations="$(psql "${RESTORE_DATABASE_URL}" -Atc 'SELECT count(*) FROM _sqlx_migrations WHERE success')"
source_tables="$(psql "${DATABASE_URL}" -Atc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE'")"
restored_tables="$(psql "${RESTORE_DATABASE_URL}" -Atc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE'")"
snapshot_query="SELECT json_build_array(
  (SELECT count(*) FROM app_metadata),
  (SELECT count(*) FROM staff_users),
  (SELECT count(*) FROM customers),
  (SELECT count(*) FROM products),
  (SELECT count(*) FROM media_assets),
  (SELECT count(*) FROM inventory_levels),
  (SELECT count(*) FROM carts),
  (SELECT count(*) FROM orders),
  (SELECT count(*) FROM order_lines),
  (SELECT count(*) FROM audit_log)
)::text"
source_snapshot="$(psql "${DATABASE_URL}" -Atc "${snapshot_query}")"
restored_snapshot="$(psql "${RESTORE_DATABASE_URL}" -Atc "${snapshot_query}")"

test "${source_migrations}" = "${restored_migrations}"
test "${source_tables}" = "${restored_tables}"
test "${source_snapshot}" = "${restored_snapshot}"
pg_restore --list "${backup_file}" >/dev/null

echo "backup restore verified: ${restored_migrations} migrations, ${restored_tables} tables, and commercial record counts"
