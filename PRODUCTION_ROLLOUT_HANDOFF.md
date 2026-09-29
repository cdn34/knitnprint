# Lean production rollout handoff

Last updated: 2026-09-29

## Purpose

This document supersedes the expensive initial production assumptions for the first low-volume launch. Production should begin close to the proven staging architecture, preserve the controls that protect orders and payments, and defer expensive high-availability components until actual load or business impact justifies them.

No production Terraform stack exists yet. This is a decision and implementation handoff, not a record of deployed production infrastructure.

Related material:

- [AWS cost discussion](./AWS_COST_CHAT_TRANSCRIPT.md)
- [Current staging Terraform](./infrastructure/terraform/staging/application/)
- [Original production infrastructure plan](./guides/production/launch-infrastructure.md)
- [Production Terraform placeholder](./infrastructure/terraform/production/README.md)
- [Staging deployment runbook](./guides/production/staging-deployment-runbook.md)

## Decisions made

The first production environment will use a lean baseline:

- one always-running ECS/Fargate application task;
- the API, storefront, and ClamAV scanner may remain consolidated as in staging initially;
- one RDS PostgreSQL `db.t4g.micro` instance with 20 GB gp3 storage;
- **Multi-AZ RDS is included** because its estimated incremental cost is acceptable;
- one internet-facing Application Load Balancer;
- Fargate tasks in public subnets with public IPs and restrictive security groups;
- no NAT gateways;
- no duplicate always-on application task at launch;
- scheduled one-shot workers instead of separate always-on workers where correctness permits;
- only essential logs, alarms, queues, backups, and secrets;
- no AWS WAF at launch unless an external requirement or observed abuse changes the decision.

The production environment must still have separate production data, credentials, buckets, task roles, secrets, DNS names, Stripe live-mode configuration, SES identity/configuration, and Terraform state. Cost reduction must not be achieved by mixing staging and production databases or secrets.

## Current staging cost baseline

AWS Cost Explorer was inspected read-only on 2026-09-29. September resources were created progressively, so the month-to-date total was not a full steady-state month.

- September actual at inspection: approximately `$61.89`, including `$11.57` tax.
- September pre-tax actual at inspection: approximately `$50.32`.
- Current steady-state projection: approximately `$95/month` before tax.
- At 23% VAT: approximately `$117/month`, before card or bank currency conversion.

Approximate full-month staging baseline:

| Component | Monthly pre-tax estimate |
| --- | ---: |
| Combined 1 vCPU / 5 GB Fargate application task | $45.80 |
| Five-minute scheduled notification worker | $1.80 |
| Application Load Balancer | $18.40 |
| Single-AZ `db.t4g.micro` and 20 GB gp3 | $15.00 |
| Public IPv4 addresses | $11.70 |
| Secrets Manager | $2.10 |
| Route 53, ECR, S3, CloudFront, SES, and other small usage | ~$0.60 |
| **Estimated steady-state total** | **~$95** |

Traffic is not currently a meaningful cost driver. The material costs are provisioned compute, RDS, the ALB, and public IPv4 hours.

## Lean production cost target

Current Ireland public rates verified through the AWS Price List API on 2026-09-29 were:

- Multi-AZ PostgreSQL `db.t4g.micro`: `$0.035/hour`.
- Multi-AZ PostgreSQL gp3: `$0.254/GB-month`.

For 730 hours and 20 GB:

| RDS configuration | Approximate monthly cost |
| --- | ---: |
| Staging-style Single-AZ micro | $14.95 |
| Production Multi-AZ micro | $30.63 |
| Increment for Multi-AZ | $15.68 |

Using a dedicated production ALB and otherwise remaining close to staging gives an initial target of approximately:

- **$110–$115/month before tax**;
- **$135–$142/month with 23% VAT**;
- variable request, email, storage, and transfer charges on top, expected to be small at launch volume.

These figures are planning estimates, not a guaranteed AWS quote. Re-run the AWS Price List and Cost Explorer checks immediately before the production plan is approved.

### Optional shared-ALB variant

Sharing an ALB could avoid roughly `$18.40/month` in ALB hours and `$7.30/month` for its two public IPv4 addresses. It would also couple the environments at the VPC, routing, certificate, access-control, and change-management layers.

The rollout baseline therefore uses a dedicated production ALB. Sharing must be a separate explicit architecture decision after confirming that the reduced isolation and larger blast radius are acceptable. Do not place production in the staging database, reuse staging secrets, or route production to staging targets merely to save this cost.

## Target architecture

```text
Internet
   |
Route 53 / external DNS
   |
Production ALB (two public subnets / two AZs)
   |
One ECS Fargate application task with a public IP
   |-- storefront
   |-- API
   `-- ClamAV
   |
Multi-AZ RDS PostgreSQL db.t4g.micro
   |-- primary in one AZ
   `-- synchronous standby in another AZ

Scheduled one-shot ECS tasks
   |-- notification outbox delivery
   |-- retention/cleanup jobs
   `-- payment reconciliation when implemented

Private S3
   |-- production media
   `-- production admin assets

CloudFront
   `-- production admin SPA
```

The ALB and VPC already span two Availability Zones in the staging model. AZs themselves are not the cost driver. Costs increase when a second billable database, application task, NAT gateway, or other resource is provisioned in the second AZ.

## Differences from staging

Production should remain close to staging, with these deliberate differences:

| Area | Staging | Lean production |
| --- | --- | --- |
| Database availability | Single-AZ | Multi-AZ |
| Database class/storage | `db.t4g.micro`, 20 GB | Same initial size |
| Database credentials | Staging roles and secrets | Separate production roles and secrets |
| Stripe | Test mode | Live mode with separate webhook secret |
| SES | Staging identity/configuration | Production identity/configuration and approved sending |
| Application tasks | One | One |
| Deployments | Cost-minimized replacement | Permit a temporary second task for safer rolling deployment |
| ALB | Dedicated staging ALB | Dedicated production ALB by default |
| NAT gateways | None | None |
| WAF | None | Deferred |
| Backups | Seven-day automated backups | At least seven days initially, plus tested restore procedure |
| Alarms | Minimal | Small essential production set |
| Indexing | Explicitly blocked | Must be enabled for the public storefront |

## Availability and accepted tradeoffs

Multi-AZ RDS protects the most important state—the orders, customers, payments, and operational records—from a single database host or AZ failure. It does not add read capacity.

One Fargate task is an accepted launch tradeoff:

- a task or AZ failure can cause application downtime while ECS replaces the task;
- there is no second continuously billed application copy;
- ECS can still restart an unhealthy task automatically;
- a second task can be introduced later without redesigning the database.

Production deployments should allow a temporary second task while a new revision becomes healthy. Unlike the staging setting of `deployment_maximum_percent = 100`, production should normally use a rolling configuration such as minimum healthy 100% and maximum 200%, subject to testing. This produces a small transient Fargate/IP charge during deployment rather than routine downtime.

No NAT gateway is required merely because two AZs are used. Public task IPs remain acceptable at launch because inbound application traffic is restricted to the ALB security group and no application container port is opened directly to the internet.

## Essential production controls that must not be deferred

The lean design is a cost decision, not permission to omit data or payment safeguards.

- Multi-AZ RDS, encrypted storage, deletion protection, TLS enforcement, and automated backups.
- A successful, documented restore test before accepting real orders.
- Separate least-privilege runtime and migration database roles.
- Separate production IAM task and execution roles.
- Separate production S3 buckets with Block Public Access and TLS-only policies.
- Stripe signature verification, idempotency, durable event handling, and reconciliation.
- Transactional email outbox delivery with retry behavior.
- Immutable, reviewed container image digests.
- HTTPS-only public endpoints and secure production cookies.
- Health checks for storefront and API targets.
- Log retention sufficient for launch troubleshooting and incident review.
- Billing alerts and a production monthly budget.
- No staging `noindex` headers, meta tags, or `robots.txt` policy on the public production storefront.

## Minimal observability set

Start with a small actionable set rather than 20–30 speculative alarms:

- ALB unhealthy production target count;
- ALB/API 5xx rate;
- ECS desired task count not equal to running task count;
- RDS CPU utilization;
- RDS freeable memory;
- RDS free storage space;
- RDS database connections;
- RDS failover/restart events;
- notification/payment dead-letter or failed-job count when those queues exist;
- account budget actual and forecast thresholds.

Container Insights can remain disabled initially. Add detailed metrics only when they answer an operational question that the base metrics and structured logs cannot answer.

## Database scaling path

Starting with Multi-AZ `db.t4g.micro` does not lock production to that size. RDS supports changing the instance class without manually migrating application data.

Monitor at least:

- sustained CPU rather than isolated peaks;
- CPU credit balance for the burstable T-class instance;
- freeable memory and swap activity;
- active and maximum database connections;
- query latency, disk queue depth, and I/O latency;
- storage remaining and storage growth.

Treat these as signals to investigate, not automatic resize commands. A practical initial review threshold is sustained CPU above roughly 70%, persistently low free memory or CPU credits, connection pressure, or user-visible query latency.

### Resize options

1. **In-place modification:** change `instance_class` and apply immediately or during a maintenance window. AWS documents that an instance-class change causes an outage.
2. **Blue/Green deployment:** create a synchronized green RDS environment, resize it, test it, and switch over. AWS states that switchovers are typically under one minute, although longer interruptions are possible.

Use Blue/Green for an established production store when its PostgreSQL configuration is supported. Budget temporarily for both database environments during the change. Keep the previous environment only for the reviewed rollback window, then remove it to stop duplicate billing.

Do not copy staging's `apply_immediately = true` behavior blindly into production. The production module should make disruptive modifications deliberate and documented.

Official references:

- [ModifyDBInstance](https://docs.aws.amazon.com/AmazonRDS/latest/APIReference/API_ModifyDBInstance.html)
- [RDS Blue/Green deployments](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/blue-green-deployments-overview.html)

## Deferred capacity and upgrade triggers

### Second application task

Defer a second always-on task until one or more of these are true:

- revenue makes a task/AZ interruption materially costly;
- traffic or background work approaches the capacity of one task;
- deployments cannot meet the required availability target;
- an uptime commitment requires redundant application capacity.

Adding a second task is the application-side high-availability step. Merely defining subnets in two AZs does not provide two running copies.

### Larger database class

Move to `db.t4g.small` or another suitable class when monitoring demonstrates memory, CPU-credit, connection, or latency pressure. Do not resize solely because production has launched.

### NAT gateways/private application subnets

Defer NAT gateways. Reconsider them if a concrete security, compliance, or network-control requirement prohibits public task IPs. Two NAT gateways would add a large fixed cost and are not necessary for basic Multi-AZ operation.

### AWS WAF

Defer the managed WAF layer at launch. Reconsider when there is observed abuse, bot traffic, application-layer attack pressure, a compliance requirement, or sufficient revenue to justify the additional protection and operating work.

Application rate limits, strict request validation, ALB security groups, CloudFront controls where applicable, and Stripe webhook signature verification remain required.

### Separate always-on workers

Prefer one-shot scheduled tasks and safely consolidated workers. Add a continuously running worker only when required latency or sustained queue depth cannot be met by the scheduled model. Preserve queue durability and idempotency; cost savings must not risk losing payment events or transactional notifications.

## Rollout sequence

Each phase must use a saved Terraform plan reviewed immediately before apply. Avoid targeted apply workflows except for an explicitly documented recovery operation; see the existing staging targeted-apply postmortem.

### Phase 1: production bootstrap

- Create the independent production Terraform state bucket and lock/state conventions.
- Create a production monthly budget and anomaly notifications.
- Establish reviewed human and deployment identities without permanent developer access keys.
- Use the `knitnprint-production-*` prefix and `Environment=production` tags.

### Phase 2: DNS, certificates, and email prerequisites

- Request production storefront/admin certificates in their required Regions.
- Publish and validate external DNS records.
- Verify the production SES identity and MAIL FROM domain.
- Confirm SES production sending access in `eu-west-1`.

### Phase 3: network and storage

- Create a dedicated production VPC following staging's two-public/two-isolated-subnet model.
- Do not create NAT gateways.
- Create ALB, application, and database security groups with the same restricted traffic flow as staging.
- Create separate production media and admin-asset buckets.
- Create production ECR repositories or explicitly document a safe shared-repository strategy based on immutable digests.

### Phase 4: database

- Create PostgreSQL `db.t4g.micro`, Multi-AZ, 20 GB gp3.
- Enable storage encryption, TLS enforcement, deletion protection, automated backups, and final snapshots.
- Create separate runtime and migration credentials through the reviewed bootstrap process.
- Run migrations with the restricted migration role.
- Execute and document a restore test before launch.

### Phase 5: application runtime

- Deploy one combined Fargate task initially, based on the known staging task layout.
- Use production-only URLs, origins, cookie behavior, secrets, buckets, SES settings, and Stripe live-mode values.
- Keep public IP assignment and restrictive ALB-only ingress.
- Configure rolling deployments to allow a temporary replacement task.
- Verify ClamAV startup time and memory headroom under the selected 1 vCPU / 5 GB task size.

### Phase 6: workers and payment reliability

- Deploy the notification worker as a scheduled one-shot task.
- Implement and validate Stripe event durability, idempotency, retry, dead-letter handling, and reconciliation before accepting payments.
- Consolidate compatible work where safe; do not combine jobs whose failure isolation or credentials require separation.
- Choose scheduler frequency from required business latency rather than copying staging's five-minute value automatically.

### Phase 7: public edge and admin

- Publish storefront DNS only after ALB targets and HTTPS behavior are healthy.
- Deploy the admin SPA through private S3 and CloudFront.
- Confirm production storefront indexing is enabled and staging indexing remains blocked.
- Validate CORS, CSP/security headers, uploads, authentication, checkout, webhooks, email, and admin operations end to end.

### Phase 8: monitoring and launch gate

- Add the minimal alarm set listed above.
- Confirm logs contain no secrets or unnecessary personal/payment data.
- Exercise task replacement and an RDS failover in a controlled pre-launch window.
- Confirm backup restore evidence, payment reconciliation, order recovery, and rollback procedures.
- Record actual daily cost after at least seven representative days and update this handoff if it differs materially from the estimate.

## Terraform implementation guidance

Production configuration should be implemented under `infrastructure/terraform/production/` rather than copying state or applying the staging root with different variables.

Reusing modules or carefully extracted shared definitions is encouraged, but preserve explicit environment boundaries. At minimum, production inputs should make these choices reviewable:

- AWS Region and Availability Zones;
- environment/resource prefix;
- immutable API and storefront image digests;
- application desired count, initially `1`;
- RDS class, initially `db.t4g.micro`;
- RDS Multi-AZ, initially `true`;
- RDS allocated storage, initially `20`;
- backup retention;
- scheduler enablement and frequency;
- ALB-sharing mode, defaulting to dedicated/disabled sharing;
- alarm thresholds and budget amount.

Do not expose secret values as Terraform variables, plans, outputs, command arguments, or state.

## Open implementation decisions

These are not blockers to agreeing on the lean architecture, but must be resolved before their respective Terraform phases:

- dedicated production ALB versus an explicitly reviewed shared-ALB design;
- exact production hostname and admin hostname;
- backup retention beyond the seven-day minimum;
- production worker schedule frequencies;
- whether the storefront remains in the combined task or moves to static/edge hosting later;
- payment queue/worker implementation details;
- production monthly budget threshold;
- the exact operational threshold for adding a second application task.

## Definition of done

The lean production rollout is complete only when:

- Terraform reports no drift after the final reviewed apply;
- one healthy production application task serves healthy storefront and API targets;
- the Multi-AZ micro database is available and a controlled failover has been observed;
- a backup restore has been successfully tested;
- production secrets and data are isolated from staging;
- Stripe live-mode webhook processing and reconciliation pass end-to-end tests;
- transactional production email works and failure events are observable;
- production indexing behavior is correct;
- the essential alarm and budget notifications are confirmed;
- the first cost review shows a credible path to the `$110–$115/month` pre-tax target;
- rollback and database-resize procedures are recorded and usable.
