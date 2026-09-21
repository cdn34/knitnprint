# Staging application infrastructure

This is the single Terraform root for the long-lived staging application. Its
`.tf` files are merged into one configuration and share the remote state key
`staging/application/terraform.tfstate`; service files are organizational, not
independent deployments.

Initial infrastructure plans and applies use the temporary
`knitnprint-administrator` SSO profile. This intentionally favors a simple,
operator-reviewed workflow over a large action-by-action infrastructure policy:
every mutation still requires a saved Terraform plan reviewed immediately
before apply, and no permanent AWS keys are used. Routine application releases
will later use a much smaller GitHub OIDC role that cannot redesign the network
or database.

The first reviewed slice creates only the network foundation:

- a dedicated `10.40.0.0/20` VPC, separate from the existing default VPC;
- two public `/24` subnets for the internet-facing ALB and lean Fargate tasks;
- two isolated `/24` database subnets without an internet route;
- one Internet Gateway and no NAT Gateway;
- three security groups: public load balancer, shared application tasks, and
  private database;
- no direct API, storefront, ClamAV, job, or database ingress from the internet.

Application containers share one task boundary because this staging environment
has fewer than five users. The load balancer is the only public entry point, and
PostgreSQL accepts traffic only from the application group. The automatically
created default VPC security group is not managed or attached.

Public IP assignment on the public subnets is intentional. Fargate tasks need
outbound access to AWS and third-party HTTPS endpoints without paying for an
always-on NAT Gateway. RDS remains in isolated subnets and will never receive a
public address.

The next reviewed slice adds:

- separate private media and admin-assets buckets with Block Public Access,
  enforced bucket ownership, AES-256 encryption, versioning, TLS-only policies,
  and 30-day cleanup of superseded versions;
- signed browser GET/HEAD/PUT CORS for the staging storefront and admin origins
  on the media bucket only;
- immutable, scan-on-push ECR repositories for API and storefront images, with
  bounded image retention.

The database slice adds one private, Single-AZ PostgreSQL 17.9
`db.t4g.micro` instance with 20 GB gp3 storage. Storage is encrypted, TLS is
forced, automated backups are retained for seven days, deletion protection and
a required final snapshot are enabled, and RDS generates and rotates the master
password through Secrets Manager. Optional Performance Insights and enhanced
monitoring remain disabled for staging.

The load-balancer slice adds the public ALB, an HTTP-to-HTTPS redirect, the
issued staging certificate, and separate IP target groups for the storefront
and `/api` traffic. Health checks use `/health` and `/api/health`; targets are
registered only when the ECS services are added. Later reviewed changes to this
same root will add runtime, edge, scheduling, and monitoring resources. Do not
create a new Terraform root for each service.

The runtime-foundation slice creates the ECS cluster without starting tasks,
three 14-day log groups, an ECS execution role, a narrowly scoped API task
role, and empty Secrets Manager containers for runtime database, migration
database, and Stripe test credentials. Secret values are populated through a
separate reviewed operational command and never enter Terraform configuration,
plans, outputs, or state.

The stopped-service slice defines one staging task containing the API,
storefront, and ClamAV sidecar. It uses the reviewed ECR image digests, a pinned
official scanner digest, 1 vCPU, and 5 GB RAM. The service starts at desired
count zero. Two additional task definitions perform the one-off database role
bootstrap and SQLx migration; only the bootstrap execution role can inject the
RDS master credential. The scripts under `scripts/staging/` populate database
and Stripe secret values through standard input so they never enter Terraform
state, shell history, command arguments, or files.
