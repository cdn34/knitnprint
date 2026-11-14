# Recovering a local database after migration renumbering

This guide covers the Google sign-in migration conflict introduced when the
feature branch was rebased onto `master`. Run one recovery option, then the
verification steps. Stop if any command fails; do not continue with later blocks.
These commands target local development PostgreSQL, not staging or production.
For deployed environments, use the [staging/production incident runbook](production/migration-conflict-recovery.md).

## What happened

The feature branch originally applied Google sign-in as migration `0059`.
Meanwhile, `master` introduced other migrations with that number and subsequent
numbers. The rebased repository now has this sequence:

| Version | Migration file |
| --- | --- |
| 0059 | `0059_variant_templates_and_skus.sql` |
| 0060 | `0060_product_sku_slugs.sql` |
| 0061 | `0061_account_email_notifications.sql` |
| 0062 | `0062_customer_favorites.sql` |
| 0063 | `0063_customer_google_signin.sql` |

There is one `0059` file in the corrected repository. The conflict is between
that file and the database's `_sqlx_migrations` record, which still identifies
Google sign-in as version `59`. SQLx checks versions and SQL checksums, so it
rejects the different SQL now assigned to that version.

Choose **option 1** if the local data is disposable. Choose **option 2** to retain
existing accounts, Google identities, products, and orders. Option 2 is specific
to the original Google migration being identical to the current `0063` SQL; its
commands verify that condition before changing the history.

Before recovering the original database, you can [rehearse either approach on a
duplicate](#rehearse-a-recovery-on-a-duplicate-database).

## Common preparation

### 1. Use the correct checkout and database

Requirements: Docker Compose, Node.js 24 with the project's npm dependencies,
Rust, and Python 3. PostgreSQL tools run inside the Compose container, so no
host `psql`, `pg_dump`, or `pg_restore` installation is needed.

Run from the checkout where you normally launch the application. On the original
development machine this is `/Users/carlosn34/Documents/projects/knitnprint`.
Compose project names normally depend on the directory name: a different Git
worktree can select a different database container.

```bash
pwd
git branch --show-current
git status --short
ls migrations/0059* migrations/0060* migrations/0061* migrations/0062* migrations/0063*
docker compose up -d --wait postgres
docker compose ps postgres
docker compose port postgres 5432
```

Confirm that the checkout contains the sequence above and that Compose selects
the intended PostgreSQL instance on local port `5432`. If your setup uses another
project, port, database, or credential, adapt the commands before running them.

Stop the API with `Ctrl+C` in its terminal. Stop any local test runners, workers,
or other processes using this database. Leave PostgreSQL running. Keep one shell
open for the rest of the procedure so the following variables remain available:

```bash
RECOVERY_DB=knitnprint
export DATABASE_URL="postgres://knitnprint:knitnprint@localhost:5432/${RECOVERY_DB}"
export MIGRATION_DATABASE_URL="$DATABASE_URL"
```

The migration binary prefers `MIGRATION_DATABASE_URL` over `DATABASE_URL`.
Setting both prevents an older shell setting from selecting a different target.
All Docker commands below use `RECOVERY_DB`; migration and seed commands use the
URLs set here.

### 2. Inspect the current history and schema

```bash
docker compose exec -T postgres psql -U knitnprint -d "$RECOVERY_DB" \
  -v ON_ERROR_STOP=1 <<'SQL'
SELECT current_database(), current_user;
SELECT version, description, success, encode(checksum, 'hex') AS checksum
FROM _sqlx_migrations
ORDER BY version;
SELECT to_regclass('public.customer_google_identities') AS google_identities,
       to_regclass('public.customer_google_challenges') AS google_challenges,
       to_regclass('public.customer_favorites') AS favorites;
SQL
```

For this conflict, expect successful versions `1`–`58`, then version `59` with
description `customer google signin`. Google tables exist; migrations `60`–`63`
are not recorded. Option 2 below refuses a different history. A failed migration
(`success = false`) needs investigation before using that option.

### 3. Back up and verify a restore

This backup contains the PostgreSQL schema, data, and migration history. It does
not contain SeaweedFS image/object bytes or local environment files. Neither
recovery option removes the SeaweedFS volume. Backups stay in the Git-ignored
`.local/` directory and should remain private.

```bash
umask 077
mkdir -p .local/backups
RECOVERY_BACKUP_FILE="$PWD/.local/backups/local-db-before-recovery-$(date -u +%Y%m%dT%H%M%SZ).dump"

docker compose exec -T postgres pg_dump -U knitnprint -d "$RECOVERY_DB" \
  --format=custom --no-owner --no-acl > "$RECOVERY_BACKUP_FILE"

test -s "$RECOVERY_BACKUP_FILE"
docker compose exec -T postgres pg_restore --list \
  < "$RECOVERY_BACKUP_FILE" > /dev/null
printf 'Backup: %s\n' "$RECOVERY_BACKUP_FILE"
```

Check that the archive can actually be restored into a separate database:

```bash
RECOVERY_VERIFY_DB="knitnprint_recovery_check_$(date -u +%Y%m%d%H%M%S)"
printf 'Verification database: %s\n' "$RECOVERY_VERIFY_DB"
docker compose exec -T postgres createdb -U knitnprint --template=template0 \
  "$RECOVERY_VERIFY_DB"
docker compose exec -T postgres pg_restore -U knitnprint \
  --dbname="$RECOVERY_VERIFY_DB" --exit-on-error --no-owner --no-acl \
  < "$RECOVERY_BACKUP_FILE"
docker compose exec -T postgres psql -U knitnprint -d "$RECOVERY_VERIFY_DB" \
  -v ON_ERROR_STOP=1 -c \
  'SELECT version, description, success FROM _sqlx_migrations ORDER BY version'
```

The restored history should match the source. This checks that the dump is
restorable without changing the original database. Keep the disposable copy
until recovery succeeds; the cleanup command at the end removes it.

Record key row counts in the original terminal for comparison after option 2:

```bash
RECOVERY_COUNTS_SQL="SELECT json_build_array(
  (SELECT count(*) FROM staff_users),
  (SELECT count(*) FROM customers),
  (SELECT count(*) FROM customer_accounts),
  (SELECT count(*) FROM customer_google_identities),
  (SELECT count(*) FROM products),
  (SELECT count(*) FROM product_variants),
  (SELECT count(*) FROM orders),
  (SELECT count(*) FROM order_lines)
)::text"
RECOVERY_BEFORE_COUNTS=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_DB" -v ON_ERROR_STOP=1 -Atc "$RECOVERY_COUNTS_SQL")
printf 'Before: %s\n' "$RECOVERY_BEFORE_COUNTS"
```

These counts assume the original Google tables exist. If the inspection found
that they do not, skip this Google-specific snapshot and investigate the state
before attempting option 2.

## Rehearse a recovery on a duplicate database

This is the method used to test this guide: take a custom-format `pg_dump`,
create a separate empty database in the same PostgreSQL container, and restore
the dump into it. The copy includes application data, schema, and
`_sqlx_migrations`, so it reproduces the original migration conflict. Only the
database name in the connection URL changes; PostgreSQL still uses port `5432`.

Complete **Common preparation** first, including its backup, restore check, and
row-count snapshot. Keep using the same terminal with `RECOVERY_DB` set to the
original database. The following commands use `RECOVERY_BACKUP_FILE` from those
steps. Leave the original API/workers stopped while comparing the two databases.

### 1. Create and populate a named test copy

This creates another copy specifically for the rehearsal, keeping the earlier
backup-verification database available as an untouched reference:

```bash
RECOVERY_SOURCE_DB="$RECOVERY_DB"
RECOVERY_SOURCE_HISTORY=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_SOURCE_DB" -v ON_ERROR_STOP=1 -Atc \
  "SELECT version, description, success, encode(checksum, 'hex')
   FROM _sqlx_migrations ORDER BY version")

RECOVERY_TEST_DB="knitnprint_recovery_test_$(date -u +%Y%m%d%H%M%S)"
test "$RECOVERY_TEST_DB" != "$RECOVERY_SOURCE_DB"
test -s "$RECOVERY_BACKUP_FILE"

docker compose exec -T postgres createdb -U knitnprint --template=template0 \
  "$RECOVERY_TEST_DB"
docker compose exec -T postgres pg_restore -U knitnprint \
  --dbname="$RECOVERY_TEST_DB" --exit-on-error --no-owner --no-acl \
  < "$RECOVERY_BACKUP_FILE"

printf 'Source database: %s\nTest database:   %s\n' \
  "$RECOVERY_SOURCE_DB" "$RECOVERY_TEST_DB"
```

If the generated name already exists, stop and choose a new test database name;
do not drop an existing database just to reuse that name. The dump/restore
method also works when the source has active connections, unlike a direct
`CREATE DATABASE ... TEMPLATE <source>` copy. Pausing writers makes the later
source-versus-backup count comparison meaningful.

### 2. Confirm that the copy starts in the same state

```bash
RECOVERY_TEST_HISTORY=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_TEST_DB" -v ON_ERROR_STOP=1 -Atc \
  "SELECT version, description, success, encode(checksum, 'hex')
   FROM _sqlx_migrations ORDER BY version")
test "$RECOVERY_SOURCE_HISTORY" = "$RECOVERY_TEST_HISTORY"

RECOVERY_TEST_BEFORE_COUNTS=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_TEST_DB" -v ON_ERROR_STOP=1 -Atc "$RECOVERY_COUNTS_SQL")
test "$RECOVERY_BEFORE_COUNTS" = "$RECOVERY_TEST_BEFORE_COUNTS"
printf 'Source counts: %s\nCopy counts:   %s\n' \
  "$RECOVERY_BEFORE_COUNTS" "$RECOVERY_TEST_BEFORE_COUNTS"
```

For the original conflict, both histories should still record Google sign-in
as `59`, with the same checksum and no recorded `60`–`63`. If comparisons fail,
inspect the output and confirm that the dump matches the intended source state.
Do not repair a copy of an unexpected database.

### 3. Enter a shell that targets only the test copy

The command below opens a child Bash shell with its own database variables.
Both URLs point to the copy, including `MIGRATION_DATABASE_URL`, which takes
precedence when running migrations. The parent shell retains the original
values, so `exit` returns to the original target afterward.

```bash
RECOVERY_TEST_URL="postgres://knitnprint:knitnprint@localhost:5432/${RECOVERY_TEST_DB}"
env RECOVERY_DB="$RECOVERY_TEST_DB" \
  RECOVERY_TEST_DB="$RECOVERY_TEST_DB" \
  RECOVERY_SOURCE_DB="$RECOVERY_SOURCE_DB" \
  RECOVERY_BACKUP_FILE="$RECOVERY_BACKUP_FILE" \
  RECOVERY_COUNTS_SQL="$RECOVERY_COUNTS_SQL" \
  RECOVERY_BEFORE_COUNTS="$RECOVERY_TEST_BEFORE_COUNTS" \
  DATABASE_URL="$RECOVERY_TEST_URL" \
  MIGRATION_DATABASE_URL="$RECOVERY_TEST_URL" \
  PS1='recovery-copy> ' \
  bash --noprofile --norc -i
```

Run the following checks **inside that child shell**, before any repair/reset
commands. The name-prefix and equality checks should all succeed:

```bash
(
  set -eu
  case "$RECOVERY_DB" in
    knitnprint_recovery_test_*) ;;
    *) printf 'Unexpected test database name\n' >&2; exit 1 ;;
  esac
  test "$RECOVERY_DB" = "$RECOVERY_TEST_DB"
  test "$RECOVERY_DB" != "$RECOVERY_SOURCE_DB"
  test "$DATABASE_URL" = "postgres://knitnprint:knitnprint@localhost:5432/${RECOVERY_DB}"
  test "$MIGRATION_DATABASE_URL" = "$DATABASE_URL"
  docker compose exec -T postgres psql -U knitnprint -d "$RECOVERY_DB" \
    -v ON_ERROR_STOP=1 -c 'SELECT current_database(), current_user'
)
```

Expect the printed database name to start with `knitnprint_recovery_test_`.
If a check fails, stop; do not paste the recovery commands into a shell still
targeting the original database. Do not source `backend/.env` in the child shell:
it can replace the test URLs with the original development URL.

### 4. Run the chosen approach against the copy

For a data-preserving rehearsal, run **steps 1–3 of
[Option 2](#option-2-repair-the-recorded-version-and-keep-existing-data)** below
in the child shell. They use its `RECOVERY_DB` and database URLs, so the history
repair and `npm run db:migrate` affect the copy.

Then run **step 1 of [Verification and restart](#verification-and-restart)**,
including the option 2 before/after count comparison. Expected results:

- Google sign-in is recorded as `63`; versions `59`–`62` are applied normally.
- A second `npm run db:migrate` succeeds without reapplying migrations.
- The saved account, Google identity, product, and order counts match the
  copy's counts before repair.

To rehearse a reset instead, run **steps 1–3 of
[Option 1](#option-1-recreate-the-local-database)** in the child shell, followed
by verification step 1. The drop/recreate command then targets the test copy.
Old row counts should not match after a reset; expect the foundation metadata
and any new owner you explicitly created instead.

Do not follow the normal **Restart the API** step during this database-only
rehearsal: that example explicitly selects the original `knitnprint` database.
No API, notification worker, or cleanup worker needs to run to verify these
migration procedures. PostgreSQL is copied; SeaweedFS objects and local
environment files are not.

To try the other approach or repeat the history repair, create a fresh test copy
from the original dump. The repaired copy no longer has the original `59`
Google record, so the history-repair guards should reject another repair there.

### 5. Return to the original shell and verify the source

Exit the child shell:

```bash
exit
```

Back in the original terminal, confirm that its variables and source history
were preserved:

```bash
test "$RECOVERY_DB" = "$RECOVERY_SOURCE_DB"
printf 'Original target restored in this shell: %s\n' "$RECOVERY_DB"

RECOVERY_SOURCE_AFTER_HISTORY=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_SOURCE_DB" -v ON_ERROR_STOP=1 -Atc \
  "SELECT version, description, success, encode(checksum, 'hex')
   FROM _sqlx_migrations ORDER BY version")
test "$RECOVERY_SOURCE_HISTORY" = "$RECOVERY_SOURCE_AFTER_HISTORY"

RECOVERY_SOURCE_AFTER_COUNTS=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_SOURCE_DB" -v ON_ERROR_STOP=1 -Atc "$RECOVERY_COUNTS_SQL")
test "$RECOVERY_BEFORE_COUNTS" = "$RECOVERY_SOURCE_AFTER_COUNTS"
```

The source should still have the original conflicting history. These checks
compare migration records and key row counts; they do not compare every value
in every table. If other processes wrote to the source during the rehearsal,
investigate those changes rather than assuming a count mismatch came from the
test copy.

### 6. Remove the rehearsal copy

In the original shell, after closing any connections to the copy:

```bash
(
  set -eu
  case "${RECOVERY_TEST_DB:-}" in
    knitnprint_recovery_test_*) ;;
    *) printf 'Refusing to drop an unexpected database name\n' >&2; exit 1 ;;
  esac
  test "$RECOVERY_TEST_DB" != "$RECOVERY_SOURCE_DB"
  docker compose exec -T postgres dropdb -U knitnprint "$RECOVERY_TEST_DB"
)
```

Keep the dump and the separate `RECOVERY_VERIFY_DB` backup-verification copy
until the actual recovery is complete. The normal verification cleanup step
removes that second copy. When ready to repair the original database, run the
chosen recovery option in the original shell; the rehearsal did not fix it.


## Option 1: recreate the local database

This removes every PostgreSQL record in `RECOVERY_DB`, including customer and
staff accounts, orders, products, sessions, and Google links. The backup is the
way to recover those records. PostgreSQL's container/volume and SeaweedFS remain.

### 1. Drop and recreate only the selected application database

The commands connect to the maintenance database `postgres`. `WITH (FORCE)`
disconnects remaining clients of the selected database. Keep the API stopped
throughout recovery.

```bash
docker compose exec -T postgres psql -U knitnprint -d postgres \
  -v ON_ERROR_STOP=1 -v recovery_db="$RECOVERY_DB" <<'SQL'
SELECT format('DROP DATABASE %I WITH (FORCE)', :'recovery_db') \gexec
SELECT format('CREATE DATABASE %I OWNER knitnprint TEMPLATE template0', :'recovery_db') \gexec
SQL
```

Do not use `docker compose down -v`: this project's Compose file also manages
the SeaweedFS volume, so that command would remove uploaded media as well.

### 2. Apply migrations and the foundation seed

```bash
npm run db:migrate
npm run db:seed
```

SQLx applies all migrations in their corrected order. The seed creates/updates
foundation metadata; it does not restore previous accounts, orders, or catalog
data, and it does not create the staff owner.

### 3. Create a development owner if needed

Replace the example values, including the password, before running:

```bash
OWNER_EMAIL=owner@example.com \
OWNER_NAME="Local store owner" \
OWNER_PASSWORD="replace-with-your-local-password" \
npm run admin:create-owner
```

The password must have at least 12 characters. Create the owner once in this
fresh database; the utility returns an error if that email already exists.
Customer accounts can be created again through the storefront.

Continue with **Verification and restart** below. Do not run option 2 afterward.

## Option 2: repair the recorded version and keep existing data

This changes the Google migration's history entry from `59` to `63`, without
re-executing its SQL. Normal migration execution then fills in `59`–`62`.
SQLx checks each recorded version individually; a recorded `63` does not prevent
it from applying these missing lower versions.

### 1. Compute the current Google migration's checksum

SQLx uses SHA-384 of the exact SQL file bytes, including whitespace. Do not edit
the migration to make a mismatch disappear.

```bash
RECOVERY_GOOGLE_CHECKSUM=$(python3 - <<'PY'
from pathlib import Path
import hashlib
print(hashlib.sha384(Path('migrations/0063_customer_google_signin.sql').read_bytes()).hexdigest())
PY
)
printf 'Google migration SHA-384: %s\n' "$RECOVERY_GOOGLE_CHECKSUM"
```

For the original conflict, this is:

```text
d4419534b8f7848f9cadd6d14a39f89a90e2285e3a2947723ab181aa45dc6fb60402090ae68c28b637687a98134fdcfe
```

The transaction below compares the file checksum with the stored one and refuses
to update a different migration. Matching checksums establish which SQL was
recorded as applied; if the schema was subsequently changed manually, review
that drift separately before continuing.

### 2. Move the original Google history entry to version 63

Run only with the API stopped and the verified backup available:

```bash
docker compose exec -T postgres psql -U knitnprint -d "$RECOVERY_DB" \
  -v ON_ERROR_STOP=1 -v google_checksum="$RECOVERY_GOOGLE_CHECKSUM" <<'SQL'
BEGIN;
SET LOCAL lock_timeout = '5s';
LOCK TABLE _sqlx_migrations IN ACCESS EXCLUSIVE MODE;
SELECT set_config('knitnprint.recovery_google_checksum', :'google_checksum', true);

DO $repair$
DECLARE
    recorded_checksum bytea;
    changed_rows integer;
BEGIN
    IF (SELECT count(*) FROM _sqlx_migrations
        WHERE version BETWEEN 1 AND 58 AND success) <> 58
       OR EXISTS (SELECT 1 FROM _sqlx_migrations WHERE NOT success)
       OR EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version < 1 OR version >= 60)
    THEN
        RAISE EXCEPTION 'Unexpected migration history; inspect it before repairing';
    END IF;

    SELECT checksum INTO recorded_checksum
    FROM _sqlx_migrations
    WHERE version = 59 AND description = 'customer google signin' AND success;

    IF recorded_checksum IS DISTINCT FROM
       decode(current_setting('knitnprint.recovery_google_checksum'), 'hex')
    THEN
        RAISE EXCEPTION 'Version 59 does not match the current Google migration';
    END IF;

    IF to_regclass('public.customer_google_identities') IS NULL
       OR to_regclass('public.customer_google_challenges') IS NULL
    THEN
        RAISE EXCEPTION 'Google schema is missing; a history-only repair is insufficient';
    END IF;

    UPDATE _sqlx_migrations SET version = 63 WHERE version = 59;
    GET DIAGNOSTICS changed_rows = ROW_COUNT;
    IF changed_rows <> 1 THEN
        RAISE EXCEPTION 'Expected to update exactly one migration row';
    END IF;
END;
$repair$;
COMMIT;

SELECT version, description, success FROM _sqlx_migrations
WHERE version >= 58 ORDER BY version;
SQL
```

Expected result: version `58`, then version `63` (`customer google signin`). Its
checksum and successful status are unchanged. The transaction rolls back if a
guard fails. Do not rerun the repair after it commits: the new `63` record makes
the original-state guard reject a second attempt.

### 3. Apply the missing migrations

```bash
npm run db:migrate
```

This applies `0059`–`0062` and skips the already recorded Google migration `0063`.
The other migrations intentionally introduce SKUs, canonical product slugs,
account notification support, and favorites. Existing records remain; product
SKU/slug values change as specified by those migrations.

No seed or owner-creation command is needed to preserve existing data. If
migration execution fails, keep the API stopped and inspect the error and
history. Individual migrations are transactional, but earlier migrations in the
same run may already have committed. Do not rerun the history repair or reverse
its version change after any of `0059`–`0062` has been applied.

## Verification and restart

### 1. Verify the history and schema

```bash
docker compose exec -T postgres psql -U knitnprint -d "$RECOVERY_DB" \
  -v ON_ERROR_STOP=1 <<'SQL'
SELECT version, description, success FROM _sqlx_migrations
WHERE version BETWEEN 59 AND 63 ORDER BY version;
SELECT to_regclass('public.variant_templates') AS variant_templates,
       to_regclass('public.product_slug_history') AS product_slug_history,
       to_regclass('public.customer_favorites') AS favorites,
       to_regclass('public.customer_google_identities') AS google_identities,
       to_regclass('public.customer_google_challenges') AS google_challenges;
SELECT table_name, column_name, is_nullable
FROM information_schema.columns
WHERE table_schema = 'public' AND (
    (table_name = 'customer_accounts' AND column_name IN ('password_hash', 'google_auto_link_disabled'))
    OR (table_name = 'customer_sessions' AND column_name IN ('reauthenticated_at', 'reauthenticated_method'))
    OR (table_name = 'notification_jobs' AND column_name = 'account_token_id')
)
ORDER BY table_name, column_name;
SQL

# A second run should succeed without applying these migrations again.
npm run db:migrate
```

All five versions should have their correct descriptions and `success = true`.
All five tables should exist. The columns listed should exist, and
`customer_accounts.password_hash` should be nullable (`YES`). Later migrations
may also be present if the repository has advanced beyond this guide.

For **option 2**, compare the saved counts before restarting the application:

```bash
RECOVERY_AFTER_COUNTS=$(docker compose exec -T postgres psql \
  -U knitnprint -d "$RECOVERY_DB" -v ON_ERROR_STOP=1 -Atc "$RECOVERY_COUNTS_SQL")
printf 'Before: %s\nAfter:  %s\n' "$RECOVERY_BEFORE_COUNTS" "$RECOVERY_AFTER_COUNTS"
test "$RECOVERY_BEFORE_COUNTS" = "$RECOVERY_AFTER_COUNTS"
```

Equal counts are a useful preservation check, not a complete comparison of every
column. Do not expect the old counts after option 1, which creates an empty
application database.

### 2. Restart the API and check it

In the API terminal, load the existing local environment and explicitly target
the recovered database. For a normal recovery this database is `knitnprint`:

```bash
set -a
source backend/.env
set +a
export APP_ENV=development
export DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint
cargo run -p knitnprint-api --bin knitnprint-api
```

In another terminal:

```bash
curl --fail --silent --show-error http://127.0.0.1:8080/api/ready
curl --fail --silent --show-error http://127.0.0.1:8080/api/account/google/config
```

Readiness should return HTTP `200`. Google config should contain the configured
client ID, or `null` if `GOOGLE_CLIENT_ID` is unset. Google setup is documented in
[Google sign-in operations](../docs/operations/google-signin.md).

Open the account page and check password/Google login, profile details, and
favorites. After option 2, also check an existing order and product. After option
1, use newly created accounts and catalog data. Run the storefront/admin dev
servers as described in the [README](../README.md) if they are not already running.

### 3. Remove only the disposable verification database

Once recovery is verified and no rehearsal process is connected:

```bash
docker compose exec -T postgres dropdb -U knitnprint "$RECOVERY_VERIFY_DB"
```

Keep the dump until you are satisfied with the recovered application. Do not
commit it.

## Restoring the backup if recovery needs to be undone

Stop the API again. Use the original terminal's `RECOVERY_DB` and
`RECOVERY_BACKUP_FILE` values; if starting a new terminal, set them explicitly
to the intended database and the dump path printed earlier. Restoring replaces
the target database and discards changes made since the backup.

```bash
test -s "$RECOVERY_BACKUP_FILE"
docker compose exec -T postgres pg_restore --list \
  < "$RECOVERY_BACKUP_FILE" > /dev/null

docker compose exec -T postgres psql -U knitnprint -d postgres \
  -v ON_ERROR_STOP=1 -v recovery_db="$RECOVERY_DB" <<'SQL'
SELECT format('DROP DATABASE %I WITH (FORCE)', :'recovery_db') \gexec
SELECT format('CREATE DATABASE %I OWNER knitnprint TEMPLATE template0', :'recovery_db') \gexec
SQL

docker compose exec -T postgres pg_restore -U knitnprint \
  --dbname="$RECOVERY_DB" --exit-on-error --no-owner --no-acl \
  < "$RECOVERY_BACKUP_FILE"

docker compose exec -T postgres psql -U knitnprint -d "$RECOVERY_DB" \
  -v ON_ERROR_STOP=1 -c \
  'SELECT version, description, success FROM _sqlx_migrations ORDER BY version'
```

This restores the original data **and its original migration conflict**. Keep
the API stopped, investigate the failed step, and then choose the appropriate
recovery route. Restoring an old dump does not itself renumber its history.

## Staging or production: preserve existing data

When the same conflict blocks a deployed release and data cannot be lost, follow
[Recovering migration conflicts in staging or production](production/migration-conflict-recovery.md).
The local reset/seed option is unsuitable for the live database. A replacement
would need a full data recovery and a controlled cutover, including every accepted
write after the backup; recreating an empty database is not that procedure.

The deployed-environment runbook covers:

- Freezing the failed release, classifying the exact error, and preserving the
  last compatible application version.
- Pausing writers and retaining incoming integration events, taking a checkpoint,
  and restoring a private rehearsal instance.
- Distinguishing an identical migration renumbering from edited SQL, conflicting
  deployment histories, and partial migrations.
- Rehearsing a guarded history repair, running the reviewed migration task, and
  rolling out compatible code before reopening mutations.
- Verifying business data and explaining why a snapshot/PITR restore alone cannot
  guarantee no lost writes.

Its commands use the actual staging layout and identify production prerequisites
explicitly: this repository does not yet contain an implemented production stack.
The guide does not authorize or execute any remote database change.

## Changes to avoid

- Do not delete the Google history row and rerun its SQL: its tables and columns
  already exist.
- Do not overwrite the stored checksum to accept unrelated SQL as migration 59.
  A history-only fix is valid here because the exact applied Google SQL matches
  the renamed file.
- Do not renumber master's migrations locally or edit already applied SQL files.
  Keep the repository's canonical sequence and repair the local history instead.
- Do not apply this repair blindly to a partially migrated or manually altered
  database. Use the inspection output and a backup to choose a state-specific fix.
