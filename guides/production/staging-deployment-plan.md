# On-demand staging deployment for `staging.knitnprint.com`

Execution record: [staging deployment runbook](./staging-deployment-runbook.md). It documents every completed application-preparation and workstation step through the verified Terraform installation.

Identity and credential model: [AWS identities and staging credentials](./aws-identity-and-staging-credentials.md).

## Summary

Deploy a complete but start/stop-capable staging environment in the existing AWS account:

```text
staging.knitnprint.com
        |
        v
Application Load Balancer
  |-- /api/* -> Rust API on ECS Fargate
  `-- /*      -> TanStack storefront on ECS Fargate

admin.staging.knitnprint.com
        |
        v
CloudFront
  |-- /api/* -> Application Load Balancer -> Rust API
  `-- /*      -> private admin S3 bucket

Rust API
  |-- private RDS PostgreSQL
  |-- private media S3 bucket
  |-- ClamAV on ECS
  |-- Stripe test mode
  `-- SES eu-west-1
```

Fixed decisions:

- AWS account: the existing shared account.
- Region: `eu-west-1`, except the CloudFront ACM certificate in `us-east-1`.
- DNS remains with the external provider.
- Infrastructure is defined with Terraform.
- Codex runs routine read-only AWS audits and local repository checks.
- You execute reviewed AWS and Terraform commands that create, update, or delete infrastructure.
- Infrastructure-changing commands are presented with intent, expected result, and recovery guidance.
- First deployment is performed locally with temporary AWS credentials, not automatically by GitHub Actions.
- Staging is started and stopped on demand.
- Database: Single-AZ standard RDS PostgreSQL with explicit start/stop.
- Initial data: migrations plus one owner account; no demo or local customer data.
- Staging sender: `no-reply@staging.knitnprint.com`.
- Stripe remains in test mode.
- Marketing infrastructure remains deferred.
- Stripe SQS/reconciliation remains a production prerequisite, not a blocker for exercising the current staging integration.

Expected costs:

- Fully running continuously: approximately `$90–115/month`.
- Mostly stopped: approximately `$35–45/month`.
- Each active day adds roughly `$2` of RDS/ECS/ClamAV compute.
- The ALB, WAF, RDS storage/backups, secrets, S3, CloudFront, and logs continue billing while staging is stopped.
- A deeper Terraform teardown can reduce the idle floor, but is not part of routine start/stop operations.

## Required application changes

Before provisioning AWS:

- Add `APP_ENV=staging`.
  - Require HTTPS origins and `DATABASE_URL`.
  - Enable secure cookies and JSON production-style logging.
  - Disable manual payments.
  - Require S3, ClamAV, and SES configuration.
  - Allow only Stripe `sk_test_` keys; reject live keys in staging.
  - Never run migrations during API startup.
- Add `EMAIL_RECIPIENT_ALLOWLIST`.
  - Required and non-empty in staging.
  - Accept normalized, exact email addresses.
  - Enforce it centrally for verification, reset, order, and fulfilment email.
  - Reject attempted delivery outside the allowlist before calling SES.
- Make the storefront deployable:
  - Add the supported TanStack Start Node/Nitro adapter.
  - Add a production start command and health endpoint.
  - Add a multi-stage storefront Dockerfile.
- Separate API addressing:
  - Keep `API_BASE_URL` server-only for SSR requests.
  - Use same-origin relative `/api/...` URLs in browser code and media URLs.
  - Prevent `127.0.0.1:8080` from appearing in browser bundles.
- Add an admin production build policy:
  - Serve the Vite SPA from private S3 through CloudFront.
  - Return `index.html` for client-side routes.
  - Add `X-Robots-Tag: noindex, nofollow` and a restrictive `robots.txt`.
- Add a shared object-storage strategy:
  - Keep the Vite development server for the admin SPA; upload staging and production admin builds to their private AWS S3 asset buckets.
  - Use MinIO for uploaded business media in development and test.
  - Require AWS S3 with task-role credentials in staging and production; reject custom endpoints and static access keys there.
  - Keep one storage API for presigned upload/download, metadata, read, write, and delete operations.
  - Route product images, category images, and future customer-customization objects through that API.
  - Use short-lived presigned PUT URLs for browser uploads.
  - Use presigned GET URLs only for private assets after authorization; keep published catalog URLs stable for caching.
- Make the backend image usable for one-off jobs:
  - Build and include API, migration, owner, notification, cleanup, and operations binaries.
  - Allow ECS task definitions to select the binary explicitly.
- Correct scheduled-command privilege issues:
  - Remove migration execution from runtime cleanup and owner commands.
  - Make media cleanup use staging AWS S3 configuration and its task role instead of local MinIO defaults.
- Harden proxy IP handling:
  - Keep direct API access restricted to the ALB.
  - Add an explicit AWS ALB trusted-proxy mode that uses the ALB-observed rightmost forwarded address.
  - Test that a client-supplied `X-Forwarded-For` value cannot spoof rate-limit identity.

Public configuration additions:

```text
APP_ENV=staging
EMAIL_RECIPIENT_ALLOWLIST=owner@example.com,tester@example.com
TRUSTED_PROXY_HOPS=1
API_BASE_URL=http://internal-api-service:8080
```

Staging runtime configuration:

```text
APP_ENV=staging
HOST=0.0.0.0
PORT=8080
TRUST_PROXY_HEADERS=true
TRUSTED_PROXY_HOPS=1
WEB_ORIGINS=https://staging.knitnprint.com,https://admin.staging.knitnprint.com
STOREFRONT_BASE_URL=https://staging.knitnprint.com

S3_REGION=eu-west-1
S3_BUCKET=knitnprint-staging-media-<aws-account-id>
MEDIA_SCANNER_ADDRESS=<private-clamav-address>:3310
MEDIA_SCAN_TIMEOUT_SECONDS=10

EMAIL_DELIVERY=ses
EMAIL_FROM=no-reply@staging.knitnprint.com
EMAIL_RECIPIENT_ALLOWLIST=<explicit-test-addresses>
AWS_REGION=eu-west-1
SES_CONFIGURATION_SET=knitnprint-staging-transactional
```

Secrets injected from Secrets Manager:

```text
DATABASE_URL
MIGRATION_DATABASE_URL
STRIPE_SECRET_KEY
STRIPE_WEBHOOK_SECRET
```

## AWS provisioning sequence

### 1. Workstation and account bootstrap

- Install a pinned Terraform release and verify its checksum; Terraform is not currently installed.
- Keep the existing AWS CLI and Docker installations.
- Sign in with the existing temporary profile using `aws login --profile knitnprint-development`.
- Verify identity with `aws sts get-caller-identity`.
- Audit the current account, SES identities, sandbox state, Route 53 zones, IAM access keys, budgets, and active resources.
- Enable root MFA and confirm that the root user has no access keys.
- Use the temporary `knitnprint-administrator` SSO profile for all manually
  reviewed Terraform plans and applies. Do not create permanent SES or
  deployment access keys or a second human staging profile.
- Later create a small GitHub OIDC role for routine image publication, ECS
  rollout, migrations, admin asset upload, and CloudFront invalidation. It must
  not have general Terraform infrastructure access.

The old `us-east-1` SES setup remains untouched during migration. AWS credentials are not intrinsically tied to an SES Region; permissions determine which regional APIs they can call. Any old long-term IAM access key created solely for SES is removed only after its usage is audited and ECS task-role delivery in Ireland succeeds.

### 2. Terraform state

Create a small bootstrap stack locally:

- Globally unique private S3 state bucket.
- Block Public Access.
- Versioning and encryption.
- TLS-only bucket policy.
- Native Terraform S3 state locking.
- Separate state keys for DNS prerequisites and staging.
- Restrict state access to the account root and the Identity Center
  `AdministratorAccess` role used by the sole human operator.

After the bootstrap apply, initialize the main stacks against the remote backend and confirm a clean plan.

### 3. Regional DNS prerequisites

Create a Terraform DNS-prerequisite stack that requests but does not manage external DNS records:

- ACM certificate in `eu-west-1` for `staging.knitnprint.com`.
- ACM certificate in `us-east-1` for `admin.staging.knitnprint.com`, required by CloudFront.
- SES identity for `staging.knitnprint.com` in `eu-west-1`.
- Easy DKIM.
- Custom MAIL FROM domain such as `bounce.staging.knitnprint.com`.
- `knitnprint-staging-transactional` configuration set.
- Account-level SES suppression enabled.
- Failure-event SNS topic for bounce, complaint, reject, and delivery-delay events.

Terraform outputs every required ACM, DKIM, MAIL FROM, MX, and TXT record. You add each record at the external DNS provider, then verify it with DNS lookup and AWS describe commands before proceeding.

Do not delete or edit the `us-east-1` SES identity. Once Ireland works, it can remain as a rollback path or be retired separately.

### 4. Network and security

Provision in two Availability Zones:

- One VPC.
- Two public subnets for ALB and lean staging Fargate tasks.
- Two isolated private subnets for RDS.
- Internet gateway.
- No NAT gateway.
- Public ALB with HTTP-to-HTTPS redirect.
- TLS 1.2+ HTTPS listener.
- WAF with AWS managed common protections and an API rate-based rule.
- ALB access logging to a lifecycle-managed S3 bucket.

Security groups:

- Load balancer: inbound HTTP/HTTPS from the internet.
- Shared application tasks: ports 3000 and 8080 from the load balancer, plus
  internal API port 8080 and ClamAV port 3310 within the same group.
- RDS: PostgreSQL only from the shared application-task group.
- No public RDS address.
- No direct public API task ingress.

### 5. Storage, database, secrets, and IAM

Provision:

- Private `knitnprint-staging-media-<account-id>` bucket with encryption, versioning, Block Public Access, TLS-only policy, quarantine/published/private prefixes, and restricted upload CORS. Production uses the entirely separate `knitnprint-production-media-<account-id>` bucket and state.
- Store product images, category images, and future customer-customization objects in that media bucket through the shared object-storage API.
- Use short-lived signed PUT requests for direct uploads and signed GET requests for authorized private objects. Serve published catalog variants through stable cacheable URLs rather than expiring signatures.
- Private `knitnprint-staging-admin-assets-<account-id>` bucket with CloudFront Origin Access Control. Production uses a separate `knitnprint-production-admin-assets-<account-id>` bucket.
- ECR repositories for API and storefront with immutable commit tags and image scanning.
- Single-AZ RDS PostgreSQL 17 `db.t4g.micro`, 20 GB gp3, encryption, forced TLS, seven-day PITR, deletion protection, and final-snapshot enforcement.
- Secrets Manager entries for migration/runtime database credentials and Stripe test credentials.
- CloudWatch log groups with 14-day staging retention.

Create separate database principals:

- RDS administrator: bootstrap only.
- Migration/schema-owner role.
- Runtime application role.
- Read-only reporting role.

Use a one-off private ECS database-bootstrap task to create those roles. Then run migrations with the migration role, apply runtime/default grants, and verify that the runtime role cannot create schemas, tables, extensions, or roles.

IAM roles:

- ECS execution role: ECR pulls, CloudWatch logs, and named secret injection.
- API task role: only staging media prefixes and SES sending from the staging identity/configuration set.
- Scheduled-task role: only the resources needed by each job.
- EventBridge Scheduler execution role: only `ecs:RunTask` for approved task definitions and required `iam:PassRole`.
- Staging-control guard role: only describe/start/stop the named staging RDS instance.
- No AWS access keys inside images, Terraform variables, GitHub secrets, or environment files.

### 6. Runtime services

Deploy:

- ECS cluster.
- Storefront service: `0.25 vCPU / 0.5 GB`, desired count 1 when running.
- API service: `0.5 vCPU / 1 GB`, desired count 1 when running.
- ClamAV service: `0.5 vCPU / 2 GB`, desired count 1 when running, pinned image digest, automatic signature updates, and health check.
- Admin assets to S3 and CloudFront.
- Internal service discovery for storefront-to-API communication.
- ALB host/path routing for storefront and `/api/*`.
- CloudFront `/api/*` behavior routed to the ALB; all other admin requests use S3.

Deploy images by immutable commit SHA and record their digests. Run migrations as a one-off task before updating services.

### 7. Initial owner and application configuration

- Do not run the development seed.
- Store a temporary owner password in Secrets Manager.
- Run the owner-creation ECS task once.
- Verify login.
- Remove the temporary owner bootstrap secret after successful creation.
- Configure store identity, currency, shipping, tax-disabled state, catalog, and inventory through the admin UI.
- Never import real production customers or payment data into staging.

### 8. Scheduled work and monitoring

Enable only while staging’s desired state is `on`:

- Transactional notification delivery every minute.
- Operations check every five minutes.
- Session cleanup daily.
- Customer-retention cleanup daily.
- Cart cleanup daily.
- Quarantined-media cleanup daily.

Do not schedule the current abandoned-payment cleanup until Stripe reconciliation is implemented; its current timeout-only cancellation behavior is unsafe.

Monitoring:

- ALB 5xx, unhealthy targets, latency, and WAF blocks.
- ECS task exits, CPU, memory, and desired/running-count mismatch.
- RDS CPU, connections, storage, and availability.
- ClamAV health.
- Scheduled task launch and non-zero exit failures.
- SES bounces, complaints, rejects, and delays.
- Notification terminal failures and stale claims.
- Backup age.
- TLS expiry.
- AWS Budget alerts at 80% and 100% of a `$120` staging budget.

Operational alerts go through a separate SNS topic to an address you confirm. This SNS topic is operational and unrelated to deferred marketing infrastructure.

## Operator-led deployment and start/stop flow

Every command is documented and executed by you. The runbook will use this progression:

1. Display the command without secrets.
2. Explain the AWS/Terraform state it changes.
3. Run a read-only precondition check.
4. You execute the mutating command.
5. Inspect and explain its output.
6. Run a read-only verification.
7. Record recovery or rollback instructions.

First deployment:

1. Authenticate and verify AWS identity.
2. Bootstrap Terraform state.
3. Apply DNS prerequisites.
4. Add and verify external DNS records.
5. Apply network, storage, IAM, RDS, ECS, CloudFront, WAF, monitoring, and budget resources.
6. Build and scan API/storefront images locally.
7. Authenticate Docker to ECR and push commit-tagged images.
8. Build admin assets and upload them to the private admin bucket.
9. Run database bootstrap.
10. Run migrations and grants verification.
11. Start ClamAV, API, and storefront.
12. Add application CNAME records at the DNS provider.
13. Create the owner.
14. Configure Stripe test webhook.
15. Run all staging acceptance tests.
16. Request SES production access in `eu-west-1` using the now-public staging URL.

Start staging:

1. Authenticate and verify account/Region.
2. Set `/knitnprint/staging/desired-state` to `on`.
3. Start RDS and wait until available.
4. Start ClamAV and wait until healthy.
5. scale API and storefront services to one.
6. Enable scheduled tasks.
7. Wait for ECS stability.
8. Check API readiness, storefront SSR, admin login, and ClamAV.
9. Report currently running resources and estimated daily cost.

Stop staging:

1. Authenticate and verify the targeted staging resources.
2. Set desired state to `off`.
3. Disable scheduled tasks.
4. Scale storefront, API, and ClamAV services to zero.
5. Wait until tasks stop.
6. Stop the RDS instance.
7. Verify no staging Fargate tasks remain and RDS is stopped.
8. Report resources that continue billing.

A small hourly guard checks the desired-state parameter. If AWS automatically restarts the stopped RDS instance after seven days while desired state remains `off`, it stops it again. It never stops RDS while desired state is `on`.

The routine stop command does not destroy data or networking. A separate, explicitly destructive teardown procedure requires a final database snapshot, media backup confirmation, typed environment confirmation, and review of the Terraform destroy plan.

## Test and acceptance plan

Application and container gates:

- Existing Rust, TypeScript, browser, security, migration, and backup tests pass.
- Both Docker images build and run locally.
- Storefront browser bundle contains no localhost API URL.
- Staging mode rejects HTTP URLs, missing database/media/email configuration, live Stripe keys, and empty recipient allowlists.
- Production continues to reject test Stripe keys.
- Runtime database credentials cannot migrate.
- Cleanup jobs operate without attempting migrations.

Infrastructure gates:

- Terraform format, validate, plan, policy scan, and clean second plan.
- No public RDS or S3 access.
- API, ClamAV, database, and task security-group paths match the intended graph.
- IAM policy simulation confirms environment-scoped access.
- ACM certificates are issued and HTTPS redirects correctly.
- No secrets appear in Terraform output, ECS definitions, logs, or GitHub.

Functional staging acceptance:

- Storefront SSR loads catalog data.
- Admin login, logout, secure cookies, and refresh work.
- `/api/*` works under both staging origins.
- Unapproved origins and spoofed forwarded IP headers are rejected or safely handled.
- Clean image upload, scan, publication, and retrieval work.
- Scanner outage fails media publication closed.
- Verification and password-reset emails reach allowlisted recipients.
- Non-allowlisted staging email is blocked before SES.
- SES bounce/complaint test events reach operational alerts.
- Stripe test checkout, successful payment, cancellation, refund, invalid signature, and duplicate webhook delivery are verified.
- The unsafe payment cleanup remains disabled.
- Notification retries and terminal failure alerts work.
- RDS snapshot/PITR and an isolated restore drill succeed.
- Start, stop, repeated start, repeated stop, and seven-day RDS guard behavior are idempotent.
- Stopped-state billing resources match the documented `$35–45/month` floor.

Documentation delivered with implementation:

- Correct every `knitnprint.com` typo to `knitnprint.com`.
- Update `guides/production/launch-infrastructure.md` with the final staging decisions and costs.
- Add a dedicated staging deployment runbook containing every command in execution order.
- Add separate troubleshooting, start/stop, rollback, SES migration, Stripe test, backup/restore, and teardown sections.

## Assumptions

- `knitnprint.com` is the authoritative domain.
- You can add CNAME, TXT, and MX records at its external DNS provider.
- One existing AWS account continues to hold staging and eventual production resources.
- Staging may be publicly reachable when started.
- The admin host is public but protected by application authentication, rate limiting, WAF, secure cookies, and `noindex`; no VPN/IP allowlist is required.
- The latest “guided raw commands” preference supersedes automatic GitHub deployment during initial provisioning. Existing GitHub Actions remain CI-only until a small routine-release OIDC role is reviewed.
- Human staging and production infrastructure changes may use temporary
  administrator SSO sessions with saved-plan review. Runtime and automated
  release roles remain least privilege; do not build a huge action-by-action
  human Terraform policy merely to avoid clearly identified temporary admin use.
- SES `us-east-1` remains operational until `eu-west-1` sending is proven; nothing is “moved” or deleted in place.
- SES production access in Ireland is requested only after the public staging site, DKIM, suppression handling, and staging recipient allowlist are working.
