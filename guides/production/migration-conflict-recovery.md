# Recovering migration conflicts in staging or production

Use this runbook when a release fails because its migration files disagree with
an existing database, and accounts, orders, payments, and other accepted writes
must be retained. A common trigger is deploying a feature migration and then
rebasing onto `master`, which has used the same number for different SQL.

The normal recovery is to correct the release/history mismatch and migrate the
**existing database forward**. Do not use the local drop/recreate/seed procedure
on an environment whose data must be kept. Creating an empty replacement and
seeding it does not recover the existing data.

This document adds incident steps to the existing
[staging migration release checklist](staging/pocket-guides/database-migration.md).
It does not provision AWS resources or execute a repair. The
[production Terraform directory](../../infrastructure/terraform/production/README.md)
currently has no implemented stack. Production follows the same decision process,
but needs its actual account, credentials, database, networking, task definitions,
and release runbook; do not obtain those by blindly replacing `staging` in names.

## 1. Freeze the failed release and classify the mismatch

Stop automatic rollouts, migration retries, and competing operator runs. Keep the
last compatible application version available until entering the maintenance
window. A failed release is not a reason to remove data or force its checksum
into `_sqlx_migrations`.

Record the failed task ARN/logs, last working image digest and commit, candidate
digest and commit, and migration histories for **each** affected environment.
The migration executable embeds SQL files at build time: editing a Git checkout
does not change an already built image.

| Evidence | Appropriate response |
| --- | --- |
| Connection, authentication, or task-placement failure with no proven history mismatch | Repair deployment/access first; do not change migration history merely because a release failed. |
| Wrong/stale image or the release accidentally edited a deployed migration | Restore the exact original deployed SQL in the release, rebuild, and put new changes in a new migration. The database may need no history edit. |
| The same exact SQL was applied under a different number | A tightly scoped history renumbering can be appropriate after checksum/schema/dependency review and a successful rehearsal. This covers the original Google `59` to `63` conflict. |
| Different SQL was applied as the same version in different environments | Establish a canonical sequence containing the required original operations. Review a separate mapping and pending-operation plan for each environment. Do not assume one UPDATE fixes every database. |
| Dirty/failed records, partial schema, data changes, or manual drift | Investigate the actual committed changes and prepare a specific forward repair. A checksum or version edit alone is insufficient. |

SQLx checks old records before reaching later migrations. Adding a migration
`0064` alone cannot bypass a conflicting `0059` checksum. Also, an unsuccessful
run may have committed earlier migrations before encountering a later failure;
inspect the history rather than assuming that nothing ran.

For this repository's particular conflict, a data-preserving mapping is valid
only if `59` records `customer google signin` and its checksum exactly matches
`0063_customer_google_signin.sql`. The missing `59`–`62` operations must also be
compatible with Google SQL having run earlier. If `59` already records variants
and SKUs, **do not relabel it as Google**.

Compare release files with a known working commit, supplying its real SHA:

```bash
: "${INCIDENT_LAST_GOOD_SHA:?Set the deployed known-good commit SHA}"
git diff "$INCIDENT_LAST_GOOD_SHA" HEAD -- migrations/
git show "$INCIDENT_LAST_GOOD_SHA:migrations/0059_customer_google_signin.sql" \
  > /tmp/knitnprint-original-google-migration.sql
python3 - <<'PY'
from pathlib import Path
import hashlib
for name in ['/tmp/knitnprint-original-google-migration.sql',
             'migrations/0063_customer_google_signin.sql']:
    print(hashlib.sha384(Path(name).read_bytes()).hexdigest(), name)
PY
```

The historical filename above is specific to the original feature release. If
that release used another file, identify the correct file from its manifest;
do not substitute a guessed version. Matching checksums identify the recorded
SQL, but do not prove the absence of later manual schema changes.

## 2. Establish the environment and private operator connection

Use a dedicated incident terminal, AWS CLI v2, jq, Terraform, PostgreSQL 17 client
tools, and Python 3. Stop on any failed command. These are concrete **staging**
values; production must use its own verified equivalents:

```bash
export AWS_PROFILE=knitnprint-administrator
export AWS_REGION=eu-west-1
export AWS_PAGER=''
INCIDENT_ENV=staging
INCIDENT_EXPECTED_ACCOUNT=739863594156
INCIDENT_DB_ID=knitnprint-staging-postgres
INCIDENT_CLUSTER=knitnprint-staging
INCIDENT_SERVICE=knitnprint-staging
INCIDENT_TF_ROOT=infrastructure/terraform/staging/application
INCIDENT_CASE="migration-conflict-$(date -u +%Y%m%dT%H%M%SZ)"
INCIDENT_DIR="$PWD/.local/migration-incidents/$INCIDENT_CASE"
umask 077
mkdir -p "$INCIDENT_DIR"

aws sso login --profile "$AWS_PROFILE"
test "$(aws sts get-caller-identity --query Account --output text)" = "$INCIDENT_EXPECTED_ACCOUNT"
aws rds describe-db-instances --db-instance-identifier "$INCIDENT_DB_ID" \
  --output json > "$INCIDENT_DIR/database-before.json"
INCIDENT_SOURCE_HOST=$(jq -er '.DBInstances[0].Endpoint.Address' \
  "$INCIDENT_DIR/database-before.json")
aws ecs describe-services --cluster "$INCIDENT_CLUSTER" --services "$INCIDENT_SERVICE" \
  --output json > "$INCIDENT_DIR/service-before.json"
INCIDENT_PRIOR_DESIRED_COUNT=$(jq -er '.services[0].desiredCount' \
  "$INCIDENT_DIR/service-before.json")
```

Verify the ARN/account, database name `knitnprint`, engine, endpoint, subnet group,
parameter group, security groups, and backup settings in that JSON. For staging,
use the existing Terraform backend/state and check these values against its
outputs. For production, record the actual equivalent configuration before
continuing; no production defaults are supplied here.

RDS is private. Arrange an approved SQL client in the VPC, or use the transport
from the [SSM tunnel guide](staging/database-read-only-tunnel.md). That guide's
tunnel host is a proposed setup, not a resource guaranteed to exist. Its personal
read-only login cannot perform this repair. The tunnel only supplies connectivity;
the controlled write needs the schema-owner migration credential. Do not make
RDS public to work around missing access or use the read-only bootstrap-query
override as a general write path.

For an already provisioned, authorized SSM host, open the source tunnel in a
separate terminal using the same AWS account/profile/region:

```bash
: "${INCIDENT_TUNNEL_INSTANCE:?Set the approved SSM instance ID}"
: "${INCIDENT_SOURCE_HOST:?Set the verified source RDS hostname}"
aws ssm start-session --target "$INCIDENT_TUNNEL_INSTANCE" \
  --document-name AWS-StartPortForwardingSessionToRemoteHost \
  --parameters "$(jq -nc --arg host "$INCIDENT_SOURCE_HOST" \
    '{host:[$host],portNumber:["5432"],localPortNumber:["15432"]}')"
```

Keep the tunnel open. Back in the incident terminal, obtain the migration login
without printing credentials. For staging, the secret JSON has `username`,
`password`, and `url` fields. Production must provide its own secret ARN/schema.
Do not enable `set -x` or put passwords in SQL files, command arguments, or logs.

```bash
INCIDENT_MIGRATION_SECRET_ARN=$(terraform -chdir="$INCIDENT_TF_ROOT" \
  output -json application_secret_arns | jq -er '.database_migration_url')
INCIDENT_CREDENTIALS=$(aws secretsmanager get-secret-value \
  --secret-id "$INCIDENT_MIGRATION_SECRET_ARN" \
  --query SecretString --output text)
export PGUSER=$(printf '%s' "$INCIDENT_CREDENTIALS" | jq -er '.username')
export PGPASSWORD=$(printf '%s' "$INCIDENT_CREDENTIALS" | jq -er '.password')
unset INCIDENT_CREDENTIALS
test "$PGUSER" = knitnprint_migration

curl --fail --silent --show-error \
  "https://truststore.pki.rds.amazonaws.com/${AWS_REGION}/${AWS_REGION}-bundle.pem" \
  --output "$INCIDENT_DIR/rds-ca.pem"
export PGHOST="$INCIDENT_SOURCE_HOST"
export PGHOSTADDR=127.0.0.1
export PGPORT=15432
export PGDATABASE=knitnprint
export PGSSLMODE=verify-full
export PGSSLROOTCERT="$INCIDENT_DIR/rds-ca.pem"
psql -X -v ON_ERROR_STOP=1 -c 'SELECT current_database(), current_user, inet_server_addr()'
```

`PGHOSTADDR` routes libpq through the tunnel while `PGHOST` supplies the actual
RDS hostname for TLS verification. These are **psql/libpq** settings; do not
assume the Rust migration executable interprets them as a tunnel connection.
Normal deployed migrations continue to use the dedicated ECS task and its
injected migration URL.

## 3. Enter a write-quiescent maintenance window

Before changing the live history or schema, stop accepting new mutations and
let existing transactions finish. Pause all API writers, admin writes, scheduled
workers, one-off jobs, and other integrations that can write to this database.
Stopping deployment alone does not stop business writes.

Arrange durable handling/replay of incoming payment and carrier webhooks and
record the last accepted event IDs. Do not acknowledge events and discard them.
If producers cannot pause or their events cannot be retained/replayed, a
zero-loss maintenance window has not been established.

For staging, the service count is `application_desired_count`; its notification
and media-cleanup schedules have literal `state = "ENABLED"` settings. Follow
the [worker pause procedure](staging-scheduled-workers.md#pause-or-remove-a-worker)
to temporarily set both relevant schedule resources to `DISABLED`, and keep
the API service at zero during repair:

```bash
export TF_VAR_application_desired_count=0
terraform -chdir="$INCIDENT_TF_ROOT" plan -out=incident-pause.tfplan
terraform -chdir="$INCIDENT_TF_ROOT" show -no-color incident-pause.tfplan
```

Review the plan before applying it. It should pause the intended service and
schedules, not replace RDS or roll out an unexpected image. The schedule edits
are required; setting the service count alone does not disable workers.

```bash
terraform -chdir="$INCIDENT_TF_ROOT" apply incident-pause.tfplan
aws ecs wait services-stable --cluster "$INCIDENT_CLUSTER" \
  --services "$INCIDENT_SERVICE"
aws ecs list-tasks --cluster "$INCIDENT_CLUSTER" --desired-status RUNNING
psql -X -v ON_ERROR_STOP=1 -c \
  "SELECT pid, usename, application_name, state, xact_start
   FROM pg_stat_activity
   WHERE datname = current_database() AND pid <> pg_backend_pid()"
```

Already running scheduled tasks are not cancelled by disabling a schedule.
Inspect remaining tasks/connections and allow writers to drain before proceeding;
do not indiscriminately kill transactions. Production needs its own service,
autoscaling, schedule, and ingress controls. There is no implemented universal
maintenance/read-only application flag in this repository.

## 4. Take a checkpoint and capture the exact current state

Create the checkpoint **after writers have drained**. Wait for it to be available:

```bash
INCIDENT_SNAPSHOT_ID="knitnprint-${INCIDENT_ENV}-before-repair-$(date -u +%Y%m%d%H%M%S)"
aws rds create-db-snapshot --db-instance-identifier "$INCIDENT_DB_ID" \
  --db-snapshot-identifier "$INCIDENT_SNAPSHOT_ID"
aws rds wait db-snapshot-available --db-snapshot-identifier "$INCIDENT_SNAPSHOT_ID"
aws rds describe-db-snapshots --db-snapshot-identifier "$INCIDENT_SNAPSHOT_ID" \
  --output json > "$INCIDENT_DIR/checkpoint.json"
```

An RDS snapshot covers the instance, not S3 object bytes. Snapshot creation on
Single-AZ RDS can briefly suspend I/O; include this in the maintenance window.
[AWS snapshot documentation](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_CreateSnapshot.html)

Capture schema and history without printing customer rows:

```bash
psql -X -v ON_ERROR_STOP=1 <<'SQL' > "$INCIDENT_DIR/history-before.txt"
BEGIN READ ONLY;
SET LOCAL statement_timeout = '30s';
SELECT current_database(), current_user;
SELECT version, description, success, encode(checksum, 'hex') AS checksum
FROM _sqlx_migrations ORDER BY version;
COMMIT;
SQL
pg_dump --schema-only --schema=public --no-owner --no-acl \
  > "$INCIDENT_DIR/schema-before.sql"
```

Record business baselines appropriate to the incident: account/Google-link
counts, products, orders/lines, payment identifiers and amounts, inventory
reservations, and pending outbox work. Use bounded queries and private incident
artifacts. Counts alone cannot prove that every value was preserved. Confirm
object-storage retention/versioning separately if any planned migration affects
media references; do not run object deletion against a shared rehearsal bucket.

## 5. Restore a private rehearsal instance and test the exact candidate

Keep production/staging connected to the original instance. Restore the checkpoint
under a **new** identifier. RDS snapshot restoration creates another instance;
it does not overwrite the source.
[AWS restore documentation](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_RestoreFromSnapshot.html)

The following creates a billable private rehearsal instance. Supply a reviewed
security group in the same VPC that permits the operator and rehearsal migration
client, and excludes normal application writers. Do not rely on default networking:

```bash
: "${INCIDENT_REHEARSAL_SG:?Set a reviewed rehearsal security-group ID}"
INCIDENT_REHEARSAL_ID="knitnprint-${INCIDENT_ENV}-rehearsal-$(date -u +%Y%m%d%H%M%S)"
test "$INCIDENT_REHEARSAL_ID" != "$INCIDENT_DB_ID"
INCIDENT_SUBNET_GROUP=$(jq -er '.DBInstances[0].DBSubnetGroup.DBSubnetGroupName' \
  "$INCIDENT_DIR/database-before.json")
INCIDENT_PARAMETER_GROUP=$(jq -er '.DBInstances[0].DBParameterGroups[0].DBParameterGroupName' \
  "$INCIDENT_DIR/database-before.json")
INCIDENT_INSTANCE_CLASS=$(jq -er '.DBInstances[0].DBInstanceClass' \
  "$INCIDENT_DIR/database-before.json")

aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier "$INCIDENT_REHEARSAL_ID" \
  --db-snapshot-identifier "$INCIDENT_SNAPSHOT_ID" \
  --db-instance-class "$INCIDENT_INSTANCE_CLASS" \
  --db-subnet-group-name "$INCIDENT_SUBNET_GROUP" \
  --db-parameter-group-name "$INCIDENT_PARAMETER_GROUP" \
  --vpc-security-group-ids "$INCIDENT_REHEARSAL_SG" \
  --no-publicly-accessible --deletion-protection \
  --tags "Key=RecoveryIncident,Value=$INCIDENT_CASE"
aws rds wait db-instance-available --db-instance-identifier "$INCIDENT_REHEARSAL_ID"
INCIDENT_REHEARSAL_HOST=$(aws rds describe-db-instances \
  --db-instance-identifier "$INCIDENT_REHEARSAL_ID" \
  --query 'DBInstances[0].Endpoint.Address' --output text)
```

Inspect the restored engine, TLS parameters, encryption, networking, and login
before using it. The [restore CLI reference](https://docs.aws.amazon.com/cli/latest/reference/rds/restore-db-instance-from-db-snapshot.html)
describes which settings need explicit selection. Do not replace the live secret's
hostname with this endpoint merely to conduct the rehearsal.

Open a second SSM tunnel using step 2's command with the rehearsal hostname and
local port `15433`. Keep it open in another terminal. In the original incident
terminal, check the copy using per-command libpq overrides; the shell's source
connection settings remain unchanged:

```bash
: "${INCIDENT_REHEARSAL_HOST:?Set the verified rehearsal endpoint}"
test "$INCIDENT_REHEARSAL_HOST" != "$INCIDENT_SOURCE_HOST"
PGHOST="$INCIDENT_REHEARSAL_HOST" PGHOSTADDR=127.0.0.1 PGPORT=15433 \
  psql -X -v ON_ERROR_STOP=1 -c 'SELECT current_database(), current_user, inet_server_addr()'
```

For an eligible Google renumbering, use step 6's SQL on this copy first. Run
the pending migrations using the **exact reviewed candidate image digest** in
a dedicated rehearsal migration task. Follow the normal migration-task pattern,
but register a separate rehearsal family whose secret injects a migration URL
for the rehearsal endpoint, never the original endpoint. Give its execution role
access only to the required rehearsal secret and its client network access only
as reviewed. Secret values must not appear in `run-task` overrides or logs.

The normal staging migration task is bound to the live staging secret: simply
rerunning it is **not** a rehearsal. A separate task definition/secret/access
configuration must be prepared and inspected first. Do not start the application,
email jobs, payment processing, or media cleanup against a copy of real data.

Verify the candidate completes, is safe to rerun, and preserves the recorded
business baselines, relationships, Google links, and payment/inventory invariants.
Review intentional changes such as SKU/slug canonicalization separately. If this
fails, stop and revise the forward repair; leave the original database intact.

## 6. Perform a narrowly scoped history repair only when justified

This is the original Google `59` to `63` case, not a generic migration repair.
The script checks the expected database/user, original history, exact SQL
checksum, and Google tables. Schema/dependency review and rehearsal remain
required: these guards cannot detect every manual drift or business invariant.

Create an incident SQL artifact from the already reviewed local repair block,
so its core guards are not retyped differently. This extracts SQL only; it does
not run Docker or change a database:

```bash
export INCIDENT_DIR
python3 - <<'PY'
from pathlib import Path
import os, re
guide = Path('guides/local-database-migration-recovery.md').read_text()
blocks = re.findall(r'```bash\n(.*?)\n```', guide, re.S)
matches = [block for block in blocks if 'DO $repair$' in block]
assert len(matches) == 1, 'Inspect the guide: repair block changed'
sql = matches[0].split("<<'SQL'\n", 1)[1].rsplit('\nSQL', 1)[0]
sql = sql.replace('BEGIN;\n', '''BEGIN;
SET LOCAL statement_timeout = '30s';
SELECT set_config('knitnprint.recovery_expected_database', :'expected_database', true);
SELECT set_config('knitnprint.recovery_expected_user', :'expected_user', true);
DO $identity$
BEGIN
    IF current_database() <> current_setting('knitnprint.recovery_expected_database')
       OR current_user <> current_setting('knitnprint.recovery_expected_user') THEN
        RAISE EXCEPTION 'Unexpected database or operator role';
    END IF;
END;
$identity$;
''', 1)
Path(os.environ['INCIDENT_DIR'], 'repair-history.sql').write_text(sql + '\n')
PY
cat "$INCIDENT_DIR/repair-history.sql"
shasum -a 256 "$INCIDENT_DIR/repair-history.sql"
```

Review and record that artifact, its checksum, operator, environment, source/candidate
digests, and the expected `59` to `63` mapping in the incident record. Compute
the migration checksum from the same reviewed candidate source:

```bash
INCIDENT_GOOGLE_CHECKSUM=$(python3 - <<'PY'
from pathlib import Path
import hashlib
print(hashlib.sha384(Path('migrations/0063_customer_google_signin.sql').read_bytes()).hexdigest())
PY
)
```

Run this against the rehearsal first, using the same migration login from the
snapshot and the copy's verified-TLS endpoint:

```bash
PGHOST="$INCIDENT_REHEARSAL_HOST" PGHOSTADDR=127.0.0.1 PGPORT=15433 \
  psql -X -v ON_ERROR_STOP=1 \
  -v expected_database=knitnprint -v expected_user=knitnprint_migration \
  -v google_checksum="$INCIDENT_GOOGLE_CHECKSUM" \
  -f "$INCIDENT_DIR/repair-history.sql" \
  > "$INCIDENT_DIR/history-repair-rehearsal-result.txt"
```

After the **complete rehearsal** in step 5 (history repair, candidate migrator,
and business-data checks) passes, explicitly confirm the original RDS
hostname/port and identity. Then repeat the approved operation against the live
database while writers remain paused:

```bash
export PGHOST="$INCIDENT_SOURCE_HOST"
export PGHOSTADDR=127.0.0.1
export PGPORT=15432
test "$PGHOST" != "$INCIDENT_REHEARSAL_HOST"
psql -X -v ON_ERROR_STOP=1 -c 'SELECT current_database(), current_user, inet_server_addr()'
psql -X -v ON_ERROR_STOP=1 \
  -v expected_database=knitnprint -v expected_user=knitnprint_migration \
  -v google_checksum="$INCIDENT_GOOGLE_CHECKSUM" \
  -f "$INCIDENT_DIR/repair-history.sql" \
  > "$INCIDENT_DIR/history-repair-result.txt"
```

The transaction rolls back on a failed guard. After a successful commit, Google
is recorded as `63`; no Google tables or customer records are recreated. If any
pending migrations have since committed, do not move that record back to `59`.

For the other cases from step 1, use their reviewed, state-specific repair
instead. Never replace the stored checksum merely to make different SQL pass,
delete migration records, mark missing migrations as already applied, or add
`IF NOT EXISTS` everywhere to conceal an unknown schema.

## 7. Run the reviewed live migration, then roll out compatible code

Use steps 4–10 of the [staging migration checklist](staging/pocket-guides/database-migration.md)
to build/pin the corrected image, register only its migration task definition,
inspect its active revision directly from ECS, and run it once. Keep the service
and workers paused; the candidate migrator must use the live migration secret,
while the rehearsal definition must retain its separate secret.

Require no task-placement failures, the `database-migration` container's exit
code `0`, and its own log stream's `database migrations applied` message. A task
reaching `STOPPED` alone is not success. AWS
[RunTask](https://docs.aws.amazon.com/cli/latest/reference/ecs/run-task.html)
can also return a `failures` array that must be checked.

Before resuming writes:

1. Re-read `_sqlx_migrations`; compare every version/checksum with the reviewed
   release manifest. Verify the expected schema and business baselines.
2. Record an unchanged source endpoint and preserved account/order/payment data.
   Review intended data transformations and any failures separately.
3. Generate a **fresh** full rollout plan; the targeted migration-task apply
   makes older saved plans stale. Inspect the image digests and keep database,
   buckets, secrets, and networking unchanged unless the incident plan requires it.
4. Keep ingress maintenance restrictions and worker pauses in place while
   starting compatible API/storefront/admin code for controlled smoke tests.
   In staging, restore `TF_VAR_application_desired_count` to the recorded
   `INCIDENT_PRIOR_DESIRED_COUNT` in a fresh reviewed rollout plan; leave schedules
   disabled until smoke tests pass. If no independent maintenance ingress exists,
   arrange controlled access before restarting the writer service. In this feature,
   an API expecting non-null passwords cannot safely replace one that created
   Google-only accounts.
5. Restore the remaining saved schedule/traffic settings through reviewed plans,
   remove the temporary maintenance restriction, and replay/reconcile buffered
   integration events by their IDs. Verify that no order/payment was omitted or
   processed twice, then monitor readiness, errors, inventory, and outbox work.

The exact production rollout controls depend on its deployed stack. Do not use
the staging service name, Terraform state, or credentials against production.
Capture all final task/image/history evidence before closing the incident.

## 8. Failure, recovery, and the no-data-loss boundary

If a guard or migration fails, stop the rollout, keep writers paused, save the
error and current history, and correct forward. A metadata mismatch often needs
no restore at all. If code rollout fails after a successful migration, an
application rollback is acceptable only when that old code supports the current
schema/data; otherwise deploy a compatible forward fix.

Do not point the application at an older snapshot/PITR instance and call that a
zero-loss rollback. Such an instance contains data only through its restore
point. Accepted writes after that point must be recovered/reconciled before any
cutover. RDS point-in-time recovery also creates a new instance, and
`LatestRestorableTime` can lag current writes.
[AWS point-in-time restore documentation](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_PIT.html)

For this reason, the preferred procedure repairs the original database while
retaining the source, checkpoint, and rehearsal evidence. If a replacement is
unavoidable, keep the original instance, define an explicit write cutoff, account
for every later database write and externally accepted event, validate the
replacement, and perform a separately reviewed cutover. Backups alone do not
establish a zero-loss guarantee. Neither a reset/seed nor restoring old history
in place is an acceptable substitute.

Remove the rehearsal instance only after the incident owner confirms it is no
longer needed, using its recorded ID/tag and a reviewed cleanup plan. Keep the
source instance and checkpoint under normal retention policies. Revoke temporary
access, close tunnels, and remove credentials from the operator shell:

```bash
unset PGPASSWORD PGUSER PGHOST PGHOSTADDR PGPORT PGDATABASE PGSSLMODE PGSSLROOTCERT
```

## Prevention for future releases

Once a migration succeeds in a shared environment, preserve its exact bytes and
identity. Integrate branch migrations before deployment, check for duplicate
numeric versions, and compare the candidate manifest with **each** target's
recorded checksums. Rebase-only renumbering is not sufficient for databases that
already applied the old number. Prefer new additive migrations for later changes.

Test upgrades from a copy of the actual previous release's schema/data as well
as a fresh empty database. Keep a release manifest of migration versions and
checksums alongside the immutable image digest. These are recommended release
gates; this guide does not claim all of them are currently automated in CI.
