# Production launch infrastructure

Last updated: 2026-08-23

This guide records the infrastructure decisions for launching the original store. The email-marketing system remains deferred. Payment event handling is treated separately because it is part of the store's core payment reliability.

## Current decisions

- Use one AWS account for now, with staging and production isolated as separate resources.
- Standardise new infrastructure in `eu-west-1` (Ireland).
- Keep the existing `us-east-1` SES setup operational until transactional email is working in `eu-west-1`.
- Defer the marketing email stack.
- Add a dedicated Stripe payment-event queue, DLQ, worker, and reconciliation process before launch.

Using one account means only one AWS root email address is required. Separate AWS accounts provide a stronger security boundary, but they are not necessary at the store's current size. Production can be moved into its own account later if the team, compliance needs, or business volume justify it.

## Estimated monthly AWS cost

The practical planning figure for staging and production together is **$220-$250 per month before tax**. This is not an AWS quote; it is a small-store estimate using current `eu-west-1` on-demand rates and the assumptions below.

| Cost area | Staging | Production | Shared | Monthly estimate |
| --- | ---: | ---: | ---: | ---: |
| ECS Fargate API, workers, and ClamAV | $15-$25 | $48-$60 | - | $63-$85 |
| PostgreSQL compute and storage | $15 | $63 | - | $78 |
| Load balancing, LCU usage, and public IPv4 | $4-$6 | $11-$15 | $27-$30 | $42-$51 |
| CloudWatch logs, metrics, and alarms | $2-$4 | $5-$8 | - | $7-$12 |
| Secrets Manager | $2-$3 | $3-$5 | - | $5-$8 |
| S3, CloudFront, Route 53, ECR, SES, and SQS | $1-$2 | $3-$8 | $2-$5 | $6-$15 |
| AWS WAF with a small custom rule set | - | - | $8-$10 | $8-$10 |
| **Expected total** | **$35-$50** | **$130-$155** | **$35-$50** | **$200-$255** |

Use **$250 per month** as the initial AWS budget so that ordinary fluctuations in logs, storage, requests, and data transfer do not immediately exceed the budget. If mainland Portuguese VAT at 23% is charged and cannot be reclaimed, a $250 pre-tax bill would become approximately **$307.50**. Confirm the account's tax treatment rather than assuming VAT will apply.

### Estimate assumptions

The estimate assumes:

- 730 billable hours in an average month.
- Static storefront and admin builds hosted through private S3 origins and CloudFront.
- One shared public Application Load Balancer with separate host rules and target groups for staging and production.
- Production API: one always-on Fargate task with 0.5 vCPU and 1 GB memory.
- Production workers: one always-on 0.25 vCPU/0.5 GB task where compatible workers are consolidated, with cleanup and reconciliation jobs run as scheduled tasks.
- Production ClamAV: one always-on 0.5 vCPU/2 GB task.
- Staging API: one 0.25 vCPU/0.5 GB task, stopped outside expected testing hours where practical.
- Staging workers and ClamAV run only when staging is being exercised. Keeping the staging scanner always on adds approximately $15-$20 per month to this estimate.
- Production PostgreSQL: `db.t4g.small`, Multi-AZ with one standby, and 50 GB gp3 storage.
- Staging PostgreSQL: `db.t4g.micro`, Single-AZ, and 20 GB gp3 storage.
- Approximately 5 GB of application logs per month, 20-30 standard CloudWatch alarms, and 12-18 managed secrets.
- Less than 20 GB of S3 media storage, modest request volume, fewer than one million SQS requests, and approximately 10,000 transactional emails per month.
- No dedicated SES IP address, paid SES add-ons, AWS Business Support, or paid Marketplace software.

Each additional minimal always-on Fargate worker at 0.25 vCPU and 0.5 GB adds approximately $9 per month. This matters if the notification and payment consumers remain separate processes instead of being safely consolidated into one task.

The production database is Multi-AZ in this estimate. Staging remains Single-AZ because it does not need the same availability target. RDS automated backups up to the provisioned database storage allowance normally do not add a separate storage charge while the database is active; additional retained backup and snapshot storage will increase the bill.

### Rates used

These rates were checked against AWS pricing data on 2026-08-23. AWS bills in USD and can change its prices, free-tier rules, and tax treatment.

| Service | `eu-west-1` rate used |
| --- | ---: |
| Fargate Linux/x86 CPU | $0.04048 per vCPU-hour |
| Fargate Linux/x86 memory | $0.004445 per GB-hour |
| RDS PostgreSQL `db.t4g.micro`, Single-AZ | $0.017 per hour |
| RDS PostgreSQL `db.t4g.small`, Multi-AZ | $0.069 per hour |
| RDS gp3 storage, Single-AZ | $0.127 per GB-month |
| RDS gp3 storage, Multi-AZ | $0.254 per GB-month |
| Application Load Balancer | $0.0252 per hour plus $0.008 per consumed LCU-hour |
| Public IPv4 address | $0.005 per address-hour |
| S3 Standard storage | $0.023 per GB-month for the first 50 TB |
| CloudWatch standard log ingestion | $0.57 per GB |
| CloudWatch standard alarm | $0.10 per alarm-metric month |
| Secrets Manager | $0.40 per secret-month plus API requests |
| SQS Standard | First one million requests per month free, then $0.40 per million in the first paid tier |
| SES transactional email | $0.10 per 1,000 with a la carte pricing, or $0.16 per 1,000 on the Essentials plan |
| Route 53 public hosted zone | $0.50 per month plus chargeable DNS queries |
| AWS WAF | $5 per web ACL, $1 per rule, and $0.60 per million requests |

The main pricing references are [AWS Fargate pricing](https://aws.amazon.com/fargate/pricing/), [RDS for PostgreSQL pricing](https://aws.amazon.com/rds/postgresql/pricing/), [Elastic Load Balancing pricing](https://aws.amazon.com/elasticloadbalancing/pricing/), [VPC and public IPv4 pricing](https://aws.amazon.com/vpc/pricing/), [S3 pricing](https://aws.amazon.com/s3/pricing/), [CloudWatch pricing](https://aws.amazon.com/cloudwatch/pricing/), [Secrets Manager pricing](https://aws.amazon.com/secrets-manager/pricing/), [SQS pricing](https://aws.amazon.com/sqs/pricing/), [SES pricing](https://aws.amazon.com/ses/pricing/), [Route 53 pricing](https://aws.amazon.com/route53/pricing/), and [AWS WAF pricing](https://aws.amazon.com/waf/pricing/).

### Lower-cost and higher-availability variants

A cost-conscious initial deployment can be kept around **$160-$200 per month** by using a Single-AZ production database, stopping staging compute outside test windows, avoiding an always-on staging scanner, and sharing the load balancer. The tradeoff is more downtime during infrastructure or Availability Zone failures. Database backups and Stripe reconciliation remain mandatory even in this variant.

A stronger high-availability layout is approximately **$300-$380 per month** and includes:

- Two production API tasks across Availability Zones.
- Always-on staging services.
- Separate staging and production load balancers.
- Private application subnets with NAT gateways in two Availability Zones.
- Additional logs, alarms, and cross-AZ traffic.

A NAT gateway is a notable fixed cost. Allow roughly $35 per gateway-month before its public IPv4 and per-GB processing charges; two Availability Zones therefore add roughly $70-$80 per month before data processing, partly offset by removing public IP addresses from application tasks. The cost-conscious estimate instead places Fargate tasks in public subnets with public IPs and security groups that accept inbound application traffic only from the load balancer. The PostgreSQL database and ClamAV port remain non-public.

### Costs not included

- Stripe transaction, dispute, refund, and currency-conversion fees.
- Domain registration or renewal.
- VAT or other taxes in the pre-tax totals.
- Engineering, incident response, or database administration time.
- Very large media storage, image transformation, or unusually high data transfer.
- AWS Business, Enterprise On-Ramp, or Enterprise Support.
- The deferred marketing stack and campaign email volume.
- Third-party monitoring, error tracking, or CDN services.

SES, SQS, and S3 are unlikely to be meaningful cost drivers at initial store volume. PostgreSQL, always-on Fargate tasks, the load balancer, public IPv4 addresses or NAT gateways, and CloudWatch are the services to watch most closely.

## Infrastructure the original store still needs

### 1. Application hosting and HTTPS

- A runtime for the Rust API.
- Hosting for the storefront and admin web applications.
- Domain/DNS configuration and a TLS certificate.
- A reverse proxy or load balancer that routes `/api/*` to the API.
- Exact `WEB_ORIGINS` values for each environment.
- Secure proxy-header handling.
- Edge rate limits.

### 2. Production PostgreSQL

- Managed PostgreSQL, or a database operated to an equivalent standard.
- Separate migration and runtime credentials.
- All migrations run successfully before the production API starts.
- Encryption at rest and in transit.
- Automated backups and point-in-time recovery.
- Scheduled backup verification and restore drills.

Staging and production must use separate databases and credentials, even while they share an AWS account.

### 3. Private object storage

The existing product-media implementation requires S3-compatible storage:

- A private bucket for each environment.
- Block Public Access enabled.
- Encryption and versioning enabled.
- Upload CORS restricted to the corresponding application origins.
- Separate quarantine and published-media prefixes.
- A workload IAM role using the supplied policies under `ops/aws`.

### 4. Malware scanner

Production uploads fail closed without the configured scanner:

- A ClamAV-compatible TCP `INSTREAM` service.
- Private network access from the API.
- Regular malware-signature updates.
- Availability and failure monitoring.

### 5. Transactional SES email

The original store uses email for:

- Account verification.
- Password resets.
- Order and fulfilment notifications.

It needs:

- A verified SES sender identity in `eu-west-1`.
- SES production access in `eu-west-1`.
- `EMAIL_DELIVERY=ses` in the production runtime.
- A workload role allowed to send only from the selected identity.
- The transactional notification worker.

Transactional notifications already use a durable PostgreSQL outbox. They do not need the deferred marketing SNS/SQS route. The Stripe payment queue described below has a different purpose and lifecycle.

### 6. Stripe and payment reliability

If card payments are enabled, the basic integration requires:

- A Stripe secret key.
- Hosted Checkout configuration.
- A public HTTPS webhook endpoint.
- A webhook signing secret.
- A restricted selection of webhook events.
- Payment, cancellation, and refund verification in Stripe test mode before live mode is enabled.

#### Existing protections

The current code already provides several useful safeguards:

- A database or processing failure returns an unsuccessful response so Stripe can retry delivery.
- Payment state, inventory state, audit data, and confirmation-notification enqueueing are updated transactionally.
- Stripe event IDs are uniquely constrained, making repeated event delivery idempotent.
- If the database commits but Stripe does not receive the response, processing the retry is safe.

Stripe retries unsuccessful live-mode webhook deliveries for up to three days. Events can also be manually resent and retrieved through the API. See [Stripe webhook delivery](https://docs.stripe.com/webhooks) and [retrieving Stripe events](https://docs.stripe.com/api/events/retrieve).

#### Gaps to close before launch

The current implementation still has important outage and reconciliation gaps:

1. An event for an unknown Checkout Session is acknowledged and not durably retained. See `backend/src/payments.rs` near the unknown-session handling.
2. The abandoned-payment cleanup can cancel an order and release inventory after a local timeout without first retrieving the Checkout Session from Stripe.
3. A delayed successful webhook could therefore arrive after the local order has been cancelled.
4. There is no scheduled process that reconciles locally pending payments against Stripe's authoritative state.

SQS alone does not solve these problems. It protects an event only after the application has received and enqueued it.

#### Required payment hardening

Use a dedicated payment queue for each environment:

```text
Stripe
  |
  v
HTTPS webhook
  |-- verify the Stripe signature
  |-- enqueue the event in stripe-events SQS
  `-- return 2xx only after enqueue succeeds
                 |
                 v
          stripe-events queue
                 |
                 v
           payment worker
             |-- apply an idempotent PostgreSQL transaction
             `-- delete the message only after the commit

          stripe-events DLQ
```

Also add a reconciliation worker:

```text
Local creating, pending, or processing payment
                   |
                   v
       Retrieve Checkout Session from Stripe
                   |
          +--------+---------+
          |                  |
          v                  v
   Stripe says paid   Stripe says unpaid/expired
   Confirm order      Cancel and release inventory
```

Before cancelling an overdue Stripe order, retrieve the Checkout Session and inspect its authoritative status. Fulfilment and reconciliation must remain idempotent. See [Stripe's Checkout fulfilment guidance](https://docs.stripe.com/checkout/fulfillment).

The launch design therefore includes:

- A dedicated `stripe-events` SQS queue and DLQ per environment.
- Fast, signature-verified webhook ingestion.
- An idempotent payment worker.
- A Stripe reconciliation worker.
- No cancellation of an overdue Stripe order based only on local time.
- Alerts for queue age, DLQ messages, and overdue local payment attempts.

A durable PostgreSQL webhook-inbox table could replace SQS in a simpler deployment. The current recommendation is SQS because the production system already uses AWS and it provides a separate recovery path. The full safeguard is Stripe retries plus SQS plus PostgreSQL idempotency plus API reconciliation and backups; the queue is not a substitute for any of the other layers.

### 7. Scheduled workers

The original store needs the following processes or scheduled commands:

- `npm run admin:deliver-notifications`: continuously, or approximately every minute.
- `npm run admin:cleanup-sessions`: on a schedule.
- `npm run admin:cleanup-customers`: on a schedule.
- `npm run admin:cleanup-carts`: on a schedule.
- `npm run admin:cleanup-payments`: on a schedule, after it has been made reconciliation-safe.
- `npm run admin:cleanup-media`: on a schedule.
- `npm run admin:check-operations`: approximately every five minutes.
- The Stripe payment-event worker, once implemented.
- The Stripe reconciliation worker, once implemented.

The store does not need `admin:consume-marketing-events` while marketing is paused.

### 8. Secrets and IAM

- A non-root deployment identity.
- Separate staging and production deployment roles.
- Separate runtime workload roles for S3, SES, and the corresponding Stripe SQS queue.
- Managed secret storage for database credentials, Stripe keys, and webhook signing secrets.
- Least-privilege policies scoped to environment-specific resources.
- MFA for privileged access.
- No AWS access keys baked into an image or committed environment file.

### 9. Monitoring and backups

- Central API and worker logs.
- Readiness and error-rate alerts.
- Database capacity and storage-pressure alerts.
- Failed transactional-notification alerts.
- Stripe webhook-ingestion alerts.
- Payment queue-age and DLQ alerts.
- Overdue-payment and reconciliation-failure alerts.
- Scanner availability alerts.
- Backup-age monitoring.
- Scheduled restore tests and alerts when they fail or become overdue.

## Infrastructure deferred with marketing

The following items are not required to launch the original store:

- Marketing SES configuration sets.
- A marketing SNS topic.
- Marketing SQS source queues.
- Marketing DLQs.
- Marketing queue policies.
- Marketing-event IAM consumer policies.
- Marketing queue and DLQ CloudWatch alarms.
- `admin:consume-marketing-events`.
- Campaign sending workers and schedulers.
- Development and staging copies of the marketing event route.

These are separate from the recommended Stripe queue and DLQ, which support core payment processing.

## Environment and AWS account layout

For the current scale, use one AWS account with fully separated environment resources:

```text
One AWS account
`-- eu-west-1
    |-- staging resources
    `-- production resources
```

Staging and production must have separate:

- IAM deployment and runtime roles.
- PostgreSQL databases and credentials.
- S3 buckets.
- Secrets.
- Stripe test/live keys and webhook endpoints.
- SES configuration sets and sender identities.
- Payment SQS queues and DLQs.
- Logging and alarms.
- Resource names and environment tags.

Example names:

```text
knitprint-staging-media
knitprint-production-media

knitprint-staging-stripe-events
knitprint-production-stripe-events

knitprint-staging-stripe-events-dlq
knitprint-production-stripe-events-dlq
```

The tradeoff is that an account-level credential compromise, quota problem, or incorrect administrator action could affect both environments. Least-privilege IAM, MFA, tested backups, explicit production guardrails, and environment-specific resource policies are proportionate mitigations for now.

## SES sandbox, domains, and staging

The website domain and the SES sender identity are related operationally but are configured separately:

- `https://staging.knitnpick.com` can be the representative website URL in the SES production-access request.
- SES domain ownership is verified using DNS records; the production web application does not need to be live first.
- Verify `staging.knitnpick.com` for staging mail.
- Verify `knitnpick.com`, or a dedicated production mail subdomain, for production mail.

Within the shared AWS account and `eu-west-1`:

1. Verify the staging and production sender identities.
2. Create separate transactional configuration sets, for example:
   - `knitprint-staging-transactional`
   - `knitprint-production-transactional`
3. Request SES production access once for the account in `eu-west-1`.
4. Select `TRANSACTIONAL` as the primary mail type.
5. Use `https://staging.knitnpick.com` as the website URL while the production site is not live.
6. Describe account-verification, password-reset, and order-notification use cases.
7. Describe the bounce and complaint suppression and monitoring process.

SES sandbox status is scoped to an AWS account and Region, not to an individual sender domain. After production access is approved in `eu-west-1`, both verified staging and production identities in that account and Region can send to unverified recipients. Staging therefore needs an application-level recipient allowlist to prevent accidental customer email.

The existing `us-east-1` SES identities and production-access status do not automatically transfer to `eu-west-1`. Keep the current setup operational during the migration, create and verify the identities again in Ireland, and submit a separate production-access request for that Region. See the [AWS SES production-access procedure](https://docs.aws.amazon.com/ses/latest/dg/request-production-access.html).

## Region recommendation

Standardise the original store's new production infrastructure in `eu-west-1`:

- PostgreSQL.
- API runtime.
- Private S3 buckets.
- Transactional SES.
- Workers and schedulers.
- SQS and DLQ for Stripe payment events.
- Logs, metrics, and alarms.

The existing `us-east-1` SES configuration can remain operational until the `eu-west-1` identity and production access are confirmed. This migration is useful for the original store and is independent of the deferred marketing stack.

## Target launch architecture

```text
HTTPS ingress
|-- Storefront/admin
`-- Rust API
    |-- PostgreSQL
    |-- private S3
    |-- ClamAV scanner
    |-- Stripe
    |-- transactional SES
    `-- stripe-events SQS/DLQ

Scheduled processes
|-- notification outbox worker
|-- payment-event worker
|-- Stripe reconciliation worker
|-- cleanup workers
`-- operational checks
```

## Pre-launch checklist

- [ ] Create environment-specific resources in `eu-west-1`.
- [ ] Configure the staging site at `staging.knitnpick.com`.
- [ ] Verify staging and production SES sender identities in `eu-west-1`.
- [ ] Request and receive SES production access in `eu-west-1`.
- [ ] Enforce a staging email-recipient allowlist.
- [ ] Provision PostgreSQL, backups, and restore testing.
- [ ] Provision private S3 storage and the ClamAV scanner.
- [ ] Configure environment-scoped deployment and runtime IAM roles.
- [ ] Implement the Stripe SQS/DLQ ingestion path.
- [ ] Implement the idempotent payment worker.
- [ ] Implement Stripe payment reconciliation before timeout-based cancellation.
- [ ] Configure scheduled workers and operational checks.
- [ ] Configure logs, alerts, and backup monitoring.
- [ ] Complete Stripe test-mode payment, cancellation, refund, retry, and reconciliation tests.
- [ ] Perform a staging deployment and production-readiness review.
