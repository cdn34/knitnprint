#!/bin/sh
# SQLx retains optional database drivers in Cargo.lock even when they are not built.
# Audit exclusions are valid only while these drivers remain outside every target graph.
set -eu

dependency_tree=$(mktemp)
trap 'rm -f "$dependency_tree"' EXIT HUP INT TERM
cargo tree --locked --workspace --target all --edges normal,build,dev --prefix none > "$dependency_tree"
if grep -E '^(sqlx-mysql|sqlx-sqlite|libsqlite3-sys|rsa) v' "$dependency_tree"; then
    echo 'PostgreSQL-only audit exclusions no longer apply: a disabled driver dependency is now enabled.' >&2
    echo 'Remove the relevant audit exclusion and resolve its advisory before enabling this dependency.' >&2
    exit 1
fi
echo 'Confirmed: optional MySQL/RSA and SQLite dependencies are not built for any target.'
