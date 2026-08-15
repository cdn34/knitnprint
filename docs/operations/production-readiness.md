# Production security and operations runbook

This runbook is the deployment gate for KnitPrint. Application controls are
verified in CI, but the infrastructure checks below must also pass in the real
production account before accepting payments or uploads.

## Hardening control matrix

| Area | Implemented control and evidence | Deployment obligation |
| --- | --- | --- |
| Authorization | Every admin handler authenticates staff and checks its named capability; owner expansion and denial paths are covered by `staff_authorization` and feature lifecycle tests. | Review grants quarterly and immediately after staff changes. |
| Ownership | Customer address and order reads resolve through the authenticated customer or opaque cart session; integration tests attempt cross-customer access. | Do not expose PostgreSQL directly to customer-facing networks. |
| Sessions/cookies | Opaque 256-bit tokens are stored as SHA-256 hashes; staff cookies are `HttpOnly`, `Secure` in production, and `SameSite=Strict`; customer cookies use `SameSite=Lax`; expiry, revocation, reset revocation, and cleanup are server-side. | Terminate HTTPS only at a trusted ingress and preserve `Set-Cookie`. |
| CSRF/CORS | Unsafe requests with an unapproved `Origin` or `Sec-Fetch-Site: cross-site` are rejected. Credentialed CORS is restricted to `WEB_ORIGINS`; wildcard origins are not accepted. | Set only the exact HTTPS storefront/admin origins. Keep `/api` same-origin at ingress. |
| Abuse controls | Staff/customer login account, IP, and global limits are persistent and concurrency-safe. Registration, verification mail, and password-reset requests have separate hashed account/IP/global limits. Request bodies are capped at 1 MiB. | Add an edge rate limit and request-size limit before the API. Alert on sustained HTTP 429 volume. |
| Media safety | Direct uploads land under `uploads-quarantine/`; size/type/head metadata, file signature, decoder allocation/dimension/pixel bounds, and a ClamAV-compatible INSTREAM scan run before normalized WebP publication. Scanner timeout/error fails closed and leaves the object quarantined; infection marks it failed and creates an immutable audit event. | Run a current scanner reachable only from the API, set `MEDIA_SCANNER_ADDRESS`, monitor failures, and update signatures continuously. |
| AWS storage | The bucket remains private, presigned writes last five minutes, derived public media is still served through ownership-aware API routes, and example TLS-only bucket/CORS/least-privilege IAM policies are under `ops/aws`. The SDK uses the standard AWS credential chain when custom local S3 keys are absent. | Enable Block Public Access, versioning, default encryption, access logging, and the supplied TLS deny. Replace all policy placeholders and attach the runtime policy to a workload role. |
| PostgreSQL permissions | Production API startup does not execute migrations and applies statement, lock, and idle-transaction timeouts. `ops/postgres/runtime-grants.sql` separates migration, runtime, and reporting privileges. | Create roles through the platform secret workflow, run migrations with `MIGRATION_DATABASE_URL`, then reapply/verify default grants. |
| Secrets | No API response or structured log contains configured credentials or account-action tokens. S3/SES prefer workload roles; runtime settings report only configured/not-configured state. | Store DB, Stripe, webhook, and any static AWS credentials in a managed secret store. Never bake them into images. Follow the rotation order below. |
| Webhooks | Stripe signatures cover the untouched body with a five-minute tolerance; provider event IDs are unique, payload hashes are immutable, and transitions lock payment/order rows. Duplicate and out-of-order events are safe. | Restrict the webhook event set, monitor signature failures, and retain the old signing secret during the provider-supported rotation overlap. |
| Audit | Sensitive staff/customer/system mutations append to a database-immutable audit log. Payment and order histories are also append-only. Tests verify actors/reasons for each operational feature. | Export immutable copies to the log/archive account and review privileged actions. |
| Backup/restore | `scripts/verify-backup-restore.sh` creates a custom-format logical backup, restores it only to an explicitly different disposable database, and compares successful migrations and table counts. CI runs this on every change. | Use encrypted managed snapshots/PITR plus daily logical backups. Run a production-like restore drill quarterly and record RPO/RTO. |
| Supply chain | CI runs npm production audit, RustSec, Trivy filesystem/secret/misconfiguration scanning, builds the non-root API container, and scans the resulting image. Builds use lockfiles. | Protect the default branch, review scanner exceptions with expiry, sign release images, and deploy by immutable digest. |
| Load/query behavior | PostgreSQL-backed contention tests verify exact limits and stock/discount serialization. k6 applies a bounded public catalog load gate with `<1%` errors and p95 `<500 ms`; production SQL has a 15-second statement timeout and operational indexes are migration-owned. | Enable `pg_stat_statements`, review top total-time/p95 queries weekly, and repeat load tests against production-sized staging data before traffic changes. |
| Retention/deletion | Dedicated bounded, row-locked workers clean sessions, tokens, customers, carts, quarantined media, and abandoned payments. Customer cleanup anonymizes identity while retaining non-personal commercial/audit history. | Schedule every worker, monitor exit status/backlog, and document jurisdiction-specific order/audit retention before launch. |
| Observability/alerts | Production emits JSON request traces with request IDs, status, and latency. Liveness/readiness are separate. `admin:check-operations` emits one JSON report and exits nonzero for actionable durable backlogs. Panics become generic JSON 500s without details. | Ship stdout centrally and configure the alert set below. Do not expose logs, health details, or database metrics publicly beyond the ingress need. |

## Deployment gate

1. Run migrations with a migration role; the production API runtime deliberately
   does not run them.
2. Apply and verify runtime/reporting grants. Confirm the runtime role cannot
   `CREATE` in the database/schema or manage roles.
3. Configure exact HTTPS `WEB_ORIGINS`, `DATABASE_URL`, Stripe, SES,
   `S3_REGION`, `S3_BUCKET`, `STOREFRONT_BASE_URL`, and
   `MEDIA_SCANNER_ADDRESS`. Prefer workload identity for AWS.
4. Apply S3 Block Public Access, versioning, encryption, TLS-only bucket policy,
   narrow admin-origin PUT CORS, and the prefix-scoped runtime IAM policy.
5. Run the full CI suite, the k6 gate against production-sized staging data,
   `admin:check-operations`, and a backup/restore drill.
6. Verify scheduled cleanup, payment reconciliation, notification delivery,
   backup, scanner update, and operational-check jobs are active and monitored.
7. Send Stripe test events and SES test mail from staging. Confirm duplicate and
   invalid webhook events, scanner outage, SES outage, and database outage all
   produce the documented safe behavior.

## Required alerts

- readiness failures for two consecutive minutes;
- HTTP 5xx above 2% for five minutes, any panic conversion, or p95 latency above
  one second for ten minutes;
- unusual HTTP 401/403 activity or sustained HTTP 429 responses;
- PostgreSQL connection saturation, statement/lock timeouts, replication lag,
  storage pressure, or a failed migration;
- any terminal notification failure, stale processing claim, overdue payment
  attempt, stale quarantine upload, detected infection, or cart/customer
  retention work still overdue after its 24-hour scheduler grace, as reported
  by `admin:check-operations`;
- Stripe signature failures, provider API error spikes, and payment/refund state
  mismatches;
- malware scanner unavailable/outdated signatures or infected upload events;
- last successful backup older than 24 hours, failed backup, failed restore
  drill, or unmet platform PITR retention;
- expiring TLS certificates, workload credentials, webhook secrets, or managed
  secret rotation deadlines.

## Secret rotation order

- Database: create a new runtime credential, grant the same runtime role,
  update the managed secret, roll all API/workers, verify old sessions drain,
  then revoke the old credential.
- Stripe API key: create/reveal the replacement, update and roll workers/API,
  run a test payment/refund, then revoke the old key.
- Stripe webhook secret: configure provider overlap when supported, deploy code
  capable of validating the active secret, send a test event, then remove the
  previous endpoint secret. If overlap is unavailable, use a brief controlled
  maintenance window and reconcile events afterward.
- AWS: rotate workload-role sessions automatically. For exceptional static
  credentials, create the second key, deploy it, verify S3/SES, then deactivate
  and delete the first key.

Never log secret values during rotation. Record who performed the change, the
time, affected deployment version, verification result, and revocation result.

## Backup and restore drill

Production should use managed encrypted snapshots and point-in-time recovery.
The repository script verifies logical portability:

```bash
DATABASE_URL=postgres://source-runtime-or-backup-role/... \
RESTORE_DATABASE_URL=postgres://isolated-empty-restore-database/... \
scripts/verify-backup-restore.sh
```

The destination must be disposable, isolated, empty, and different from the
source. After the script passes, run readiness plus a read-only staff/catalog
smoke test against the restored database, record elapsed restore time and the
newest recovered transaction, then destroy the isolated destination through the
platform's approved process.

## Incident first actions

For suspected credential theft, disable/revoke the credential, preserve logs,
rotate it, and reconcile affected audit/payment/storage records. For suspicious
media, keep the quarantine prefix private, stop publication completion if the
scanner is unreliable, update signatures, rescan retained objects, and inspect
all `media.scan_rejected` events. For payment disagreement, stop manual state
changes, preserve webhook payload hashes, query Stripe by provider identifiers,
and reconcile through an audited repair rather than editing immutable history.
