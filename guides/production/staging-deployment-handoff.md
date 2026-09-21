# Staging deployment handoff

Last updated: 2026-09-15

## Resume this Codex session

From the repository root, run:

```bash
codex resume 01a02fdd-0202-7cf1-9c04-215a36827923
```

If that session cannot be resumed, start a new session and ask Codex to read this file together with:

- [Staging deployment runbook](./staging-deployment-runbook.md)
- [Staging deployment plan](./staging-deployment-plan.md)
- [AWS identities and staging credentials](./aws-identity-and-staging-credentials.md)
- [Terraform installation on WSL Ubuntu](./terraform-wsl-installation.md)

## Working agreement

- Codex runs routine read-only AWS inspection and audit commands.
- Carlos runs AWS commands that create, update, or delete infrastructure.
- Carlos runs Terraform commands that inspect, plan, or change infrastructure after Codex prepares and reviews the configuration.
- Codex may run local repository inspection, editing, formatting, build, and test commands.
- Give Carlos only commands that materially control or change infrastructure, grouped when it is safe to do so.
- Explain what each infrastructure command establishes and review its expected effect before any mutation.
- Do not create, modify, or delete AWS resources without an explicit reviewed step.

## Repository checkpoint

```text
Repository:     /home/carlosn34/projects/test-p
Branch:         carlosnogueira/staging-deployment
HEAD:           e82660e fix: harden deployment runtime images
Terraform:      v1.16.0, installed from the HashiCorp APT repository and held
Workload Region: eu-west-1
Identity Region: us-east-1
```

The worktree contains documentation changes and untracked guide files. Preserve them and inspect `git status --short` before staging or committing anything. The existing modified `guides/production/staging-deployment-plan.md` belongs to the ongoing deployment work.

The Terraform bootstrap is deployed and its state is stored in the dedicated
versioned S3 backend. The application network, private object storage, ECR
repositories, and private PostgreSQL database are deployed and verified.

## Completed application and workstation preparation

The application deployment-preparation work through commit `f7aa77b` is complete and locally verified. The detailed commit sequence, acceptance results, container checks, and Terraform installation repair are recorded in the staging deployment runbook.

Terraform was installed successfully on WSL Ubuntu as:

```text
Terraform v1.16.0
on linux_amd64
Debian package: terraform 1.16.0-1 amd64
APT hold: active
```

## Completed AWS identity bootstrap

IAM Identity Center is an organization instance in `us-east-1` with **Multi-account permissions** available.

The selected administrative identity is:

```text
Identity Center user:  knitnprint-admin
Identity Center group: Admin
Permission set:        AdministratorAccess
CLI profile:           knitnprint-administrator
SSO session:           knitnprint
SSO Region:            us-east-1
Client Region:         eu-west-1
Output format:         json
```

The user is a member of the Identity Center group `Admin` (description: `Admins for KnitnPrint AWS environment`). The group is assigned to the management account with the predefined `AdministratorAccess` permission set. This temporary human session is used for all reviewed Terraform plans and applies. Routine releases will instead use a small GitHub OIDC role. The redundant `KnitNPrintStagingDeployer` permission set has been removed through Terraform.

The local profile was configured with:

```bash
aws configure sso --profile knitnprint-administrator
```

The SSO session name must not be left blank. Leaving it blank caused the legacy-format warning; configuration was restarted with `knitnprint` as the session name. The browser authorization was completed as `knitnprint-admin`.

## Verified AWS preflight

These commands completed successfully:

```bash
aws sso login --profile knitnprint-administrator

aws sts get-caller-identity \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager

aws configure get region \
  --profile knitnprint-administrator

aws iam get-account-summary \
  --profile knitnprint-administrator \
  --output json \
  --no-cli-pager
```

Observed evidence:

- Caller identity is `knitnprint-admin` through an assumed `AWSReservedSSO_AdministratorAccess_...` role.
- The caller is not root and is not an IAM user.
- The configured client Region is `eu-west-1`.
- There are zero IAM users.
- Root/account MFA is enabled.
- Root has no access keys.
- The root password is present, which is normal.
- The exact AWS account ID was verified interactively but is intentionally not copied into this repository guide.

SSO credentials are temporary. If a later command reports an expired session, run:

```bash
aws sso login --profile knitnprint-administrator
```

## Completed read-only IAM inventory

The following read-only inventories completed:

```bash
aws iam list-groups \
  --profile knitnprint-administrator \
  --output json \
  --no-cli-pager

aws iam list-policies \
  --scope Local \
  --profile knitnprint-administrator \
  --output json \
  --no-cli-pager

aws iam list-open-id-connect-providers \
  --profile knitnprint-administrator \
  --output json \
  --no-cli-pager

aws iam list-saml-providers \
  --profile knitnprint-administrator \
  --output json \
  --no-cli-pager
```

### Findings

Two historical IAM groups exist:

```text
Admins           created 2021-02-02
knitnpick-dev    created 2024-10-07
```

These are traditional IAM groups and are separate from the Identity Center group `Admin`. Direct inspection confirmed that both IAM groups contain no users; their attached and inline policies are recorded below.

Five customer-managed IAM policies exist:

```text
AWSLambdaBasicExecutionRole-61b69b3e-8ca2-41ca-af0b-e6c4247c8067
  attachment count: 0

knitnpick-dev-access-policy
  attachment count: 0

ses-mail-manager-role-20260721-215520
  attachment count: 1

ses-mail-manager-role-20260721-215408
  attachment count: 1

ses-mail-manager-role-20260721-215252
  attachment count: 1
```

The first two are unattached and appear historical. Each SES Mail Manager policy is attached to its matching Mail Manager role. Preserve all five until legacy cleanup is handled as a separate reviewed task.

There are no IAM OpenID Connect providers. GitHub Actions OIDC federation has therefore not yet been configured.

One SAML provider exists with a name matching:

```text
AWSSSO_..._DO_NOT_DELETE
```

This is expected infrastructure for IAM Identity Center. Do not delete or modify it.

The role inventory contains:

- the expected IAM Identity Center `AWSReservedSSO_AdministratorAccess_...` role;
- AWS service-linked roles for SES, Organizations, Resource Explorer, SSO, Support, and Trusted Advisor;
- three historical SES Mail Manager roles;
- no existing KnitNPrint staging deployer or workload role.

The historical `Admins` IAM group contains no users and has no inline policies. It still has these AWS-managed policies attached:

```text
AdministratorAccess
AdministratorAccess-AWSElasticBeanstalk
AdministratorAccess-Amplify
```

Because the group has no members, those policies currently grant access to nobody. Leave the group and policies unchanged until the broader legacy-resource audit is complete.

The historical `knitnpick-dev` IAM group contains no users and has no attached or inline policies. It is inert, but should likewise remain unchanged until cleanup is handled as a separate reviewed task.

## Completed SES and Mail Manager audit

The regional SES inventory found:

- `us-east-1` sending is enabled and healthy but remains in the SES sandbox (`ProductionAccess=false`, 200 messages per 24 hours, one message per second);
- the `us-east-1` account-level suppression list covers both bounces and complaints;
- `knitnprint.com` is a verified, sending-enabled domain identity in `us-east-1`;
- several other historical domain and individual-address identities exist in `us-east-1`; their addresses are intentionally not copied into this repository;
- `us-east-1` has one legacy configuration set named `my-first-configuration-set`;
- `eu-west-1` contains a `knitnprint.com` domain identity in `FAILED` verification state with sending disabled;
- `eu-west-1` has no configuration sets.

The Ireland account and identity inspection confirmed:

- `eu-west-1` sending is enabled and healthy but remains in the SES sandbox (`ProductionAccess=false`, 200 messages per 24 hours, one message per second);
- the Ireland account-level suppression list covers both bounces and complaints;
- the failed identity reports `HOST_NOT_FOUND` for both identity and DKIM verification;
- Namecheap's authoritative nameservers were found, but the expected Ireland Easy DKIM DNS names were not;
- DKIM signing is enabled and there is no custom MAIL FROM domain configured.

This is a DNS-publication failure, not an IAM, credential, or SES account-health failure. Leave the failed apex identity unchanged. The Terraform DNS-prerequisite stack will later request the planned `staging.knitnprint.com` identity and output its regional records for manual publication in Namecheap.

The legacy `us-east-1` configuration set `my-first-configuration-set` has sending and reputation metrics enabled, but no tags, delivery controls, or event destinations were found. It therefore does not implement the planned staging bounce/complaint event route and should not be copied to Ireland. Leave it unchanged until legacy cleanup is reviewed separately.

The three historical SES Mail Manager roles were created within minutes of each other on 2026-07-21. Each:

- trusts only the `ses.amazonaws.com` service principal;
- restricts assumption to this AWS account;
- restricts assumption to one exact `us-east-1` Mail Manager rule set;
- has no recorded last use.

They are not general-purpose credentials and are unrelated to the planned ECS task roles. The absence of recorded use suggests abandoned setup experiments; their associated resources and policies are mapped below, but any cleanup remains a separate decision.

The Mail Manager resource inventory found:

- three rule sets matching the three role trust policies;
- one public authenticated (`AUTH`) ingress point in `ACTIVE` state;
- no Mail Manager archives.

Current [SES pricing](https://aws.amazon.com/ses/pricing/) includes an authenticated ingress endpoint without the `$50` monthly fixed endpoint charge; that charge applies to Open and mTLS ingress endpoints. The `ESSENTIALS` plan also has no fixed monthly account charge and bills for messages processed. With no archives and no recorded role use, this setup should not have an idle fixed charge. It remains unrelated to staging and should be left unchanged until legacy cleanup is reviewed separately.

The active endpoint inspection further established that it:

- uses the newest of the three rule-set/traffic-policy pairs;
- is a public IPv4 authenticated endpoint using the FIPS TLS policy;
- has an `ALLOW` default traffic-policy action, which still applies behind SMTP authentication rather than creating an unauthenticated open relay;
- has no Mail Manager outbound relay resources.

The two older rule-set/traffic-policy pairs are not attached to the active endpoint and appear to be setup leftovers. All three rule sets perform the same `Send` action through their matching, source-restricted SES role. All three traffic policies have no statements and use `ALLOW` as their default. No legacy IAM or SES resource was modified or deleted.

## Completed governance, DNS, and active-resource audit

The account is the management account of an AWS Organization with all features enabled.

DNS and edge findings:

- Route 53 contains one unrelated `carlosnogueira.dev` public hosted zone.
- Route 53 Domains contains no registered domains.
- `knitnprint.com` DNS remains externally managed.
- ACM contains no certificates in either `eu-west-1` or `us-east-1`.
- CloudFront contains no distributions.

Cost-control findings:

- No AWS Budget exists.
- Cost Anomaly Detection has a confirmed daily email subscription.
- Its alert requires both at least `$100` absolute impact and at least `40%` impact, so it is not a substitute for a small staging monthly budget.
- No customer-created CloudTrail trail exists; normal CloudTrail event history remains available independently.

Direct inspection found no active deployment or billable application resources in either `eu-west-1` or `us-east-1` across:

- EC2 instances, NAT gateways, Elastic IP addresses, and load balancers;
- RDS instances or clusters;
- ECS clusters, ECR repositories, Lambda functions, or App Runner services;
- ElastiCache or MemoryDB clusters;
- DynamoDB tables, OpenSearch domains, or EKS clusters;
- active CloudFormation stacks;
- Secrets Manager secrets or SSM parameters;
- CloudWatch log groups, SNS topics, SQS queues, EFS file systems, or Backup vaults.

Resource Explorer has one local index and view in `us-east-1`. Its indexed objects were defaults plus the IAM, SES, Route 53, and cost-anomaly resources described above. Ireland was verified directly because the index is not an account-wide aggregator.

Three historical S3 buckets remain:

```text
knitnpick-dev
knitnpick-production
knitnpick-stage
```

All three are in `eu-west-2`, empty, unversioned, encrypted with S3-managed encryption, and protected by all four S3 Block Public Access settings. Empty buckets have no storage charge. Preserve them until cleanup is handled as a separate reviewed task; they do not conflict with correctly named KnitNPrint staging resources.

## Exact stopping point

The audit, bootstrap, DNS/SES prerequisite apply, and publication of all seven
external DNS validation records are complete. Both ACM certificates, the SES
identity, DKIM, and custom MAIL FROM have validated successfully. A single
application Terraform root now contains the applied and verified network
foundation, private storage/ECR foundation, database, load balancer, and
no-compute ECS runtime foundation. Immutable API and storefront images from
commit `e82660e` are published to private ECR and passed scan-on-push with zero
findings. The next work is:

1. Configure and populate the Stripe test secret, then start one service task and
   register healthy ALB targets.
2. Add CloudFront, external application DNS, and focused monitoring.
3. Complete public acceptance checks and request SES production access in
   `eu-west-1` after the public staging site,
   recipient allowlist, and failure handling are working.

## Object-storage requirement added during bootstrap work

The application storage boundary was refactored after the AWS audit:

- `backend/src/object_storage.rs` now defines the shared strategy interface for presigned upload/download, metadata, read, write, and delete operations.
- Development and test select MinIO through its S3-compatible endpoint and local credentials.
- Staging and production select AWS S3, require `S3_REGION` and `S3_BUCKET`, reject `S3_ENDPOINT` and static S3 access keys, and rely on the workload IAM role.
- Existing product upload, processing, delivery, and cleanup code now uses the shared interface rather than the AWS SDK client directly.
- Five-minute presigned PUT uploads remain the browser upload mechanism.
- Presigned GET support is available for future private category/customer-customization objects after authorization.
- Public catalog images retain stable `/api/media/...` URLs for caching instead of embedding expiring S3 signatures.
- The admin SPA itself remains Vite-served in development; staging and production builds use a separate private S3 bucket behind CloudFront.

Six focused storage tests pass, including deterministic checks that presigned PUT and GET requests target the local MinIO bucket with five-minute AWS Signature V4 URLs and that deployed environments reject a bucket named for the other environment. The full Rust workspace test suite and strict Clippy check also pass. The existing Docker services were stopped, so no persistent local volumes were started or changed for a live MinIO regression run.

The initial Terraform bootstrap configuration has also been added under `infrastructure/terraform/staging/bootstrap`. It is formatted and uses the signed HashiCorp AWS provider `6.62.0`. The first saved plan was deliberately discarded before apply because its deployer policy draft granted broad wildcard access to regional services that could also affect future production resources. The policy was narrowed to identity discovery and exact access to the dedicated state bucket's `staging/` prefix. The replacement plan passed human and machine-readable review: it resolved the real Identity Center group `Admin`, proposed exactly 10 creates and one data-source read, and contained no updates, replacements, or deletions. Its SHA-256 was `b31e4f820c41cdcbdb937e2a1052e451b576a0dd6c3f27cbb7e2764d86c64c0d`.

That exact plan was successfully applied on 2026-09-02: 10 resources were added, none changed, and none destroyed. Live read-only verification confirmed the state bucket `knitnprint-staging-terraform-state-739863594156` in `eu-west-1`, enabled versioning, AES-256 default encryption, Bucket Owner Enforced ownership, all four Block Public Access settings, TLS-only access, and the approved-principals restriction. The `$120` monthly budget is healthy with actual 80%, actual 100%, and forecast 100% notifications. Identity Center contains the hardened `KnitNPrintStagingDeployer` policy, its assignment to group ID `547884f8-1001-704b-5597-ccfd6c3d3254`, and the provisioned role `AWSReservedSSO_KnitNPrintStagingDeployer_e84c298508d0a223`. The bootstrap state was then migrated successfully to `staging/bootstrap/terraform.tfstate` in the verified bucket. The object is encrypted with AES-256 and has S3 version ID `en8wIBhJxUTtjmPeCsQafNOsAV4P17Tv`; no active `.tflock` object remains. Terraform can list all 10 managed resources from the remote state, and a post-migration plan reported no changes. Production will use an independent Terraform root, state bucket, workload/release roles, and `knitnprint-production-*` resources.

The former local `knitnprint-staging` profile reused SSO session `knitnprint`
and assumed `AWSReservedSSO_KnitNPrintStagingDeployer_e84c298508d0a223` in
account `739863594156`. Read-only checks with that profile successfully located
and read the remote state object before the redundant profile and permission
set were removed. The next root, `infrastructure/terraform/staging/dns`,
has been added, initialized against `staging/dns/terraform.tfstate`, formatted, and
validated against AWS provider `6.62.0`. It requests the two staging ACM
certificates and the Ireland SES/SNS prerequisites but leaves all Namecheap
records as outputs. Its first plan proposed exactly nine additions and no
changes or deletions. It was not applied: review found that SES
configuration-set reputation metrics would create separately billed CloudWatch
metrics and that certificate export should be prohibited explicitly. That plan
was renamed `staging-dns.pre-cost-hardening.stale-do-not-apply.tfplan`; it must
never be applied. The configuration now disables reputation metrics, retains
the SNS failure-event destination, and sets both ACM certificates to `export =
"DISABLED"`.

The bootstrap deployer
policy now contains the exact service permissions required by this root. Its
saved update plan proposes only two in-place changes—the permission-set
description and inline policy—without additions or deletions. The 4,679-byte
policy produced no AWS Access Analyzer findings. The approved plan SHA-256 is
`8cb56b785428dc6e571eeccde09000522ae4856ced0f16ec98709951842622cb`.
That exact plan was applied successfully: no resources were added or destroyed,
and the two expected permission-set resources changed in place. Read-only
verification found the same 13 statement IDs in both Identity Center and the
provisioned `AwsSSOInlinePolicy` on the generated IAM role. The restricted
profile can read the Ireland SES account settings, and a subsequent bootstrap
plan reported no changes before the latest hardening. The certificate-export
guard plan had SHA-256
`ab6d5e86927fb2bb261a8aaabcfea0f5560162c9e89b4d016013f8a5e25403cb` and
updated only the deployer inline policy: zero resources were added or destroyed
and one changed in place. Live verification confirmed that both the Identity
Center permission set and its provisioned `AwsSSOInlinePolicy` now require
`acm:Export = DISABLED` for certificate requests. IAM simulation returned
`allowed` for the exact staging request with export disabled and
`implicitDeny` for the same request with export enabled. The replacement DNS
plan passed human-readable and machine-readable review with exactly nine
creates, one deferred policy-document read, no updates, deletions, or
replacements, and no secret inputs or outputs. It explicitly disables
certificate export and SES reputation metrics. Its SHA-256 is
`26fe8f78d00f880d7dcc0d497f49b205f962a2dad5a275b4d146af5993439f0a`.
Administrator-profile collision checks found no existing resources with the
planned ACM domain names, SES identity/configuration-set name, or SNS topic
name. Broad list calls remain intentionally unavailable to the restricted
staging role. The exact reviewed plan was applied successfully: nine resources
were added, none changed, and none destroyed. Live checks confirmed both
certificates are non-exportable and pending DNS validation, Easy DKIM uses
`dkim.amazonses.com`, custom MAIL FROM is pending, reputation metrics are off,
the SES failure event destination targets the restricted SNS topic, and all
resources have the staging tags. The encrypted remote state object has S3
version ID `_kJJ9ti8Sn0z3iHng2zLC3LwdtLUVsVS`; a post-apply refresh plan reports
no changes. Public authoritative DNS contains none of the pending record names.

## Published staging validation records

The following records were added to the `knitnprint.com` zone in Namecheap with
Automatic TTL. Hosts are relative to `knitnprint.com`. Do not add the storefront
or admin application CNAMEs yet because their AWS endpoints do not exist.

| Type | Host | Value | Priority |
| --- | --- | --- | --- |
| CNAME | `_960e472a522141a26014dda9cafef4b6.admin.staging` | `_fd536fe8b4e35a4570860d4b0e8962fc.jkddzztszm.acm-validations.aws` | — |
| CNAME | `_6c95f4ae888bd0eedaeecdf4caa64f3b.staging` | `_d2348c51d8454d8e16defccb9e147d48.jkddzztszm.acm-validations.aws` | — |
| CNAME | `k2cmtjxttlfsssf32tue7rdacrhpyucv._domainkey.staging` | `k2cmtjxttlfsssf32tue7rdacrhpyucv.dkim.amazonses.com` | — |
| CNAME | `cc2eoyxwziuobpmeehde5ijv7v5lx7fl._domainkey.staging` | `cc2eoyxwziuobpmeehde5ijv7v5lx7fl.dkim.amazonses.com` | — |
| CNAME | `p3rp6lam23ouz6juescu4pp3gs2r2c4n._domainkey.staging` | `p3rp6lam23ouz6juescu4pp3gs2r2c4n.dkim.amazonses.com` | — |
| MX | `bounce.staging` | `feedback-smtp.eu-west-1.amazonses.com` | `10` |
| TXT | `bounce.staging` | `v=spf1 include:amazonses.com ~all` | — |

Both authoritative Namecheap nameservers return all seven expected values. Live
AWS checks on 2026-09-07 report `ISSUED` and validation `SUCCESS` for both ACM
certificates, plus SES identity verification `SUCCESS`, DKIM `SUCCESS`, custom
MAIL FROM `SUCCESS`, and sending enabled for the identity.

The environment naming contract is `knitnprint-<environment>-<purpose>`. The planned staging media, admin-assets, and state buckets are distinct from their future production equivalents. A large EC2 action-by-action expansion was drafted locally, reached roughly 9.4 KB across its policy documents, and was removed without being applied because it made a one-person infrastructure workflow harder to understand and maintain. All human-reviewed Terraform operations now use the temporary administrator SSO profile and saved plans. Terraform removed the redundant staging permission-set assignment, inline policy, and permission set, and removed its allowance from the state-bucket policy: zero resources were added, one changed in place, and three were destroyed. Runtime roles stay least privilege, and a future GitHub OIDC release role will be limited to publishing images/assets, running migrations, updating ECS services, and invalidating CloudFront. Production follows the same pragmatic separation rather than recreating the oversized human policy. The backend refuses to start if staging is configured with a production-prefixed bucket or production with a staging-prefixed bucket. Shared implementation belongs in Terraform modules later; environment roots and state must remain separate rather than using Terraform workspaces.

Post-cleanup checks found only `AdministratorAccess` in the Identity Center
permission-set list. The local AWS CLI profile list no longer contains
`knitnprint-staging`, the state-bucket policy allows only account root and the
Identity Center administrator role, and a bootstrap refresh plan reports no
changes.

Removing the unapplied EC2 policy draft itself changed no AWS resources. After
the separate, reviewed permission-set cleanup apply, all three Terraform roots
validated successfully and a live bootstrap refresh plan reported `No changes`.

The DNS backend was then reconfigured to use `knitnprint-administrator`; its
live refresh plan reported `No changes`. The application root was initialized
successfully against the separate empty state key
`staging/application/terraform.tfstate`.

The first application plan contains exactly 17 creates, no updates or
deletions, and no sensitive values. It creates only the `10.40.0.0/20` VPC,
Internet Gateway, two public subnets, two isolated database subnets, their route
tables and associations, and three security groups for the load balancer,
application tasks, and database. It creates no compute, load balancer, database,
NAT Gateway, storage, or allocated public IP. The reviewed saved plan is
`staging-network.tfplan` with SHA-256
`046105c919ec24b0e2ff8177c5096364047829260775551c59f966e1bb1c1521`.

That plan applied successfully: 17 resources were added, none changed, and none
destroyed. The VPC is `vpc-06bc08d6224acf925`; public subnets are
`subnet-002fa2e87423dbf6b` and `subnet-048a0ab536c3c65aa`; isolated database
subnets are `subnet-09ef327e37512e019` and
`subnet-0ee3677e276ace8bc`. Live route inspection confirmed only the public
subnets have the Internet Gateway default route. Security groups are
`sg-0bff5d9437bbeeab8` for the load balancer,
`sg-0e6e9ce76ef5f6a4e` for application tasks, and
`sg-055b1f31d10679955` for PostgreSQL. A post-apply plan reports no changes.
The encrypted application state object has S3 version ID
`zBxR8uHTCLl4rI7KRlFt2pdnBUirYbQ4`.

The next saved application plan adds the storage and container-registry
foundation. Machine-readable review found exactly 19 creates, 17 unchanged
network resources, two deferred TLS-policy document reads, no updates or
deletions, and no sensitive values. It creates the two exact private staging
buckets with public access blocked, Bucket Owner Enforced ownership, AES-256
encryption, versioning, 30-day noncurrent-version cleanup, seven-day incomplete
multipart cleanup, and TLS-only policies. Only the media bucket receives CORS,
limited to signed `GET`, `HEAD`, and `PUT` requests from the two staging web
origins. The API and storefront ECR repositories use AES-256 encryption,
immutable tags, scan-on-push, seven-day untagged cleanup, and a newest-20 image
cap. The reviewed `staging-storage.tfplan` SHA-256 is
`aa382624e5dcfc9bcf2bf6a8007a36600b3c5bcbe2ad085358e4dd4508d77199`.

That exact plan applied successfully: 19 resources were added, none changed,
and none destroyed. Live checks confirmed all four Block Public Access flags,
Bucket Owner Enforced ownership, AES-256 encryption, enabled versioning,
30-day noncurrent-version expiration, seven-day incomplete-upload cleanup,
TLS-only bucket policies, and the exact media CORS rules. Both ECR repositories
are AES-256 encrypted, immutable, scan on push, and have matching bounded
retention policies. A post-apply Terraform plan reports no changes. The current
encrypted application state version is
`IC9WiELHFBVXXiU5Zoy_Besk1pOxsxlA`.

The saved database plan contains exactly three creates and 36 unchanged
resources, with no update, replacement, or deletion. It creates a two-subnet
RDS subnet group, a PostgreSQL 17 parameter group forcing TLS, and one private
Single-AZ PostgreSQL 17.9 `db.t4g.micro` instance with 20 GB encrypted gp3
storage. The plan contains no password: RDS generates and rotates the master
credential in Secrets Manager. Seven-day backups, deletion protection, and a
required final snapshot are enabled; optional paid monitoring is disabled. The
reviewed `staging-database.tfplan` SHA-256 is
`4cae055dbb10ea7d71d04b183c7535e15b85cf7d79970b831eb76fffea75c5a4`.

That exact database plan applied successfully: three resources were added,
none changed, and none destroyed. The instance is available at
`knitnprint-staging-postgres.cdugyku62w1w.eu-west-1.rds.amazonaws.com:5432` in
VPC `vpc-06bc08d6224acf925`. Live checks confirmed PostgreSQL 17.9 on
`db.t4g.micro`, 20 GB gp3, private access, encryption, Single-AZ placement,
seven-day backups, deletion protection, and `rds.force_ssl = 1`. The RDS-owned
Secrets Manager secret has rotation enabled; its value was not read. AWS
normalizes the static TLS parameter's apply method to `pending-reboot`, so the
Terraform declaration was aligned with that provider/API value. Validation and
the post-apply plan now pass with no changes. The encrypted application state
object advanced to version `LNibNicwWFxl0rzHxz_GnYcJLMBKj7qi`.

The reviewed load-balancer plan had SHA-256
`d3d149f9a213d94a98221b9253c596aa232a5a0967076ccfbac0f2ad0f439011`
and applied exactly six additions with no changes or deletions. The public ALB
is active at `knitnprint-staging-473653489.eu-west-1.elb.amazonaws.com` across
the two public subnets. Port 80 redirects to HTTPS, port 443 uses the issued
`staging.knitnprint.com` certificate and the TLS 1.2/1.3 policy, `/api` and
`/api/*` route to the API target group, and all other paths route to the
storefront target group. Both target groups are intentionally empty until ECS
services are added. Live AWS inspection matched the plan and the post-apply
Terraform plan reported no changes. The encrypted application state object
advanced to version `2KXYlGC4U3PYXTIECqq3EmZCQVyqJAP8`.

The reviewed runtime-foundation plan had SHA-256
`8997d979142896712c76c223eb99b4ea38bcfb376b5763bfd329e4d8203244e9`
and applied exactly 12 additions with no changes or deletions. It created the
`knitnprint-staging` ECS cluster, three 14-day CloudWatch log groups, the ECS
execution and API task roles, and three application secret containers. The
execution role can read only those three application secrets; it cannot read
the RDS master secret. The API task role is limited to the staging media bucket
and email sent through the staging SES identity and configuration set. Live
checks confirmed Container Insights is disabled, the cluster has zero services
and tasks, all log groups are empty, and every application secret has zero
versions. No secret value was retrieved. The post-apply Terraform plan reported
no changes. The encrypted application state object advanced to version
`KduRXdKi5J3esAHixappXc_2OsETXxK8`.

The API and storefront deployment images were rebuilt from commit
`e82660e316202d420964b5b38d33aeeb26722407`, smoke-tested locally, and pushed
under that immutable full-SHA tag. Their tagged ECR OCI index digests are
`sha256:3bfdd65ea94b55306745e061cf043173894dfd712d664ff54062f62f3536f224`
for the API and
`sha256:507addeef2118fa26fd1a7c547275348fd577ddcb82ad2721f4123cb68405029`
for the storefront. ECR scanned their Linux manifests
`sha256:2e872ca6d5aa9addfd4a13a993172162c4b702b240ab6b40952bfc149cd26e2c`
and
`sha256:71ec5a605378d5a0ffa807bb5f56b1e3099c0ace054e35257fb54ebb8fd23c84`;
both scans completed successfully with zero findings at every severity. The
older images are retained as rollback artifacts and have not been deleted.

The next saved Terraform plan is
`infrastructure/terraform/staging/application/staging-ecs-definitions.tfplan`.
It has SHA-256
`d0004bbfd70e8cd2ec45e9a3d73b27e90038dd183182c58b014c0e6d8891a8ad`.
Machine-readable inspection found exactly eight creates and 57 no-ops, with no
updates, deletions, replacements, or sensitive variables. It adds a zero-count
ECS service, the combined API/storefront/ClamAV task definition, stopped
database bootstrap and migration task definitions, one bootstrap-only
execution role and policy, and one 14-day bootstrap log group. Since the
service desired count is zero, applying it starts no Fargate compute.

That exact plan applied successfully with eight additions and no changes or
deletions. Live inspection found an active service with zero desired, running,
or pending tasks, and the post-apply Terraform plan reported no changes. The
database secret containers remained empty. The encrypted remote application
state advanced to version `WynP39LwVqv7kxdDqCFS0ODwUkWx0I7I`.

Generate and store the database credentials without displaying them:

```bash
./scripts/staging/populate-database-secrets.sh
```

That script completed on 2026-09-14. It generated the credentials without user
input, as intended. Read-only inspection confirmed one `AWSCURRENT` version for
both the migration and runtime database secrets; no secret value was read or
displayed. The next action is the one-off ECS database bootstrap task.

The first bootstrap task stopped safely with exit code 3 before applying
grants because the script explicitly requested `NOSUPERUSER`, an attribute
change the RDS administrator cannot perform. The redundant attribute clauses
were removed; PostgreSQL's role defaults already provide the restricted
attributes. The reviewed `staging-database-bootstrap-fix.tfplan` replaces only
the bootstrap task-definition revision, leaves the zero-count service
unchanged, contains no sensitive variables, and has SHA-256
`aae289d08696e79c38a978cabb69c7de181c3e5eb43a3824446721eb21e72e08`.

The correction plan applied successfully and active task-definition revision 2
was used by bootstrap task `bde9ba3ebe444f24bc48c9b5536d3433`.
It exited zero after executing the full role/grant/revoke sequence. A separate
read-only task verified that migration/runtime are login roles, reporting is
not, and none can become superuser, create databases or roles, replicate, or
bypass row-level security. Terraform reports no drift. The encrypted remote
application state advanced to version `z2R4dZok7q8ekCYECxAuIsyLpqTgzeEP`.
The next action is the one-off SQLx migration task.

Migration task `1f70eb24b19742a7adaa48e582e9bda7` subsequently exited zero.
Its log reported `database migrations applied`; a separate read-only VPC query
confirmed 24 successful migration rows, zero failures, and 50 public tables.
The next blocker before starting the zero-count ECS service is the Stripe
test-mode API key and webhook signing secret.

### Resume checkpoint: 2026-09-15

The migration task's stopped status and exit code zero were rechecked before
the session interruption. On resuming, the read-only CloudWatch/Stripe secret
metadata check stopped because the administrator SSO token had expired; no AWS
resources or secret values were changed or read. Renew the session with:

```bash
aws sso login --profile knitnprint-administrator
```

Codex should then inspect Stripe secret version metadata (not its value). If
the secret has not been populated, prepare Stripe test-mode credentials and run:

```bash
./scripts/staging/populate-stripe-secret.sh
```

The script prompts without echoing for the test API key (`sk_test_...`) and
endpoint signing secret (`whsec_...`). Do not paste either into chat. The
webhook URL is `https://staging.knitnprint.com/api/payments/stripe/webhook`.
Keep the application service at desired count zero until the credentials are
ready; review a saved Terraform plan before starting it.

After Carlos renewed the session, read-only verification succeeded: the Stripe
secret returned no versions, and the active ECS service had desired/running/
pending counts of zero with application task-definition revision 1. Stripe
credential population is therefore still required; no infrastructure changed.

On 2026-09-17, the population script created Stripe secret version
`f806ad61-b787-4e18-87a3-99e6054a0f29` with the `AWSCURRENT` stage. Only
version metadata was inspected; neither credential was read. Both staging ACM
certificates are `ISSUED`, and the SES staging identity, DKIM, and custom MAIL
FROM statuses are all successful. Public DNS still has no application record
for `staging.knitnprint.com`. The declared application desired count was
advanced from zero to one; validate and review the resulting Terraform plan
before applying it. After the service is healthy, add the public staging DNS
record and test the application before sending a Stripe webhook test event.

The saved startup plan `staging-service-start.tfplan` was reviewed. It contains
exactly one in-place change: ECS desired count `0 -> 1`; there are no additions
or destructions. Its SHA-256 is
`92bab196965fbbc9176695b0205084fcc864e0baf3ab88324e01199d49f60962`.
Apply that exact plan, then inspect ECS task/container status, target-group
health, and CloudWatch logs before publishing application DNS.

The startup plan applied successfully. Live verification found desired/running/
pending counts `1/1/0`, a completed ECS rollout, a healthy combined task, and
healthy API and storefront ALB targets. API, storefront, and ClamAV logs show
clean startup. Direct HTTPS testing through the ALB returned `200` for the SSR
storefront and an `ok` API health response. A post-apply Terraform plan reports
no drift.

Do not publish the application DNS record yet. The live storefront image does
not return `X-Robots-Tag`, contains no robots meta element, and returns `404`
for `/robots.txt`. Add staging-only indexing protection and deploy its updated
image first. Remaining launch work is: indexing protection; storefront DNS;
the private-S3/CloudFront admin application and admin DNS; initial owner/store
configuration; and end-to-end staging acceptance including Stripe webhooks,
uploads, authentication, and email. Scheduled jobs, minimal operational alarms,
SES production-access approval, and the small GitHub OIDC release workflow are
follow-up operational work rather than blockers for the first private staging
review.

Staging indexing protection is implemented in commit `37c63ed` using the
runtime `APP_ENV` value. Staging responses include `X-Robots-Tag: noindex,
nofollow, noarchive, nosnippet`, SSR HTML includes the equivalent robots meta
element, and `/robots.txt` returns `Disallow: /`. A production-mode check
confirmed the header/meta are absent and `/robots.txt` returns `Allow: /`.
Typecheck, production build, direct compiled-server checks, and a Docker smoke
test all passed. Local image `knitnprint-storefront:37c63ed` has image ID
`sha256:eaeb06f4a8eca93b264456f665ddeb310f9a42ff87429650e55fd30bd2396589`
and runs as the non-root `knitnprint` user. The image is not yet in ECR and the
live ECS service still runs the previous storefront digest. Push the commit-tag
image next, record its registry digest, update Terraform, and deploy it before
publishing public application DNS.

The commit-tagged image was pushed successfully. ECR reports OCI index digest
`sha256:eaeb06f4a8eca93b264456f665ddeb310f9a42ff87429650e55fd30bd2396589`;
its Linux/AMD64 child manifest is
`sha256:0d16082adc9b605ebccd71f8dc634e976cc7a8b64b96d9c4608024c74a234ebf`.
Basic scan-on-push completed on that platform manifest with zero findings. The
Terraform storefront digest is pinned to the index digest, and the task
definition now sets `APP_ENV=staging`. Prepare and review the rollout plan next.

The saved `staging-noindex-rollout.tfplan` was reviewed directly in JSON. It
changes exactly the application task definition (new revision) and ECS service
(in-place task-definition update): one add, one change, and one old-revision
destroy/deregistration. Desired count remains one. The plan contains the
verified storefront index digest and `APP_ENV=staging`, with no unrelated
infrastructure changes or secret values. Its SHA-256 is
`73670eb988b80bd0fdaaeb4125ae7a9aba2e149681aeddeb8ac9afd5555f6377`.

That exact plan applied successfully and registered application task-definition
revision 2. ECS reports desired/running/pending `1/1/0`, a completed rollout,
and a healthy task; ClamAV is healthy and both API/storefront ALB targets are
healthy. Live HTTPS checks through the ALB returned storefront `200`, the full
`X-Robots-Tag`, the equivalent SSR robots meta element, `Disallow: /` from
`/robots.txt`, and an `ok` API health response. Recent API, storefront, and
ClamAV logs show clean startup. A post-apply Terraform plan reports no drift.
Staging indexing protection is complete; storefront DNS publication is next.

Namecheap now publishes `staging.knitnprint.com` as a CNAME to the staging ALB.
Authoritative DNS returns the expected target, and HTTPS verification using the
published host returned storefront `200`, the complete noindex controls, and an
`ok` API health response. Local recursive DNS briefly lagged the authoritative
answer during propagation; this did not indicate an application failure.

Admin edge configuration is drafted locally. The admin SPA now obtains its
storefront link from `VITE_STOREFRONT_URL` (commit `3963e44`), and the reviewed
staging build embeds `https://staging.knitnprint.com`. Typecheck/build passed;
the output retains its robots meta element and `Disallow: /`. Terraform now
defines CloudFront OAC for the existing private admin-assets bucket, a scoped
CloudFront-only bucket read grant, an `admin.staging.knitnprint.com`
distribution using the issued `us-east-1` certificate, staging noindex and
security response headers, and an uncached `/api/*` origin routed to
`https://staging.knitnprint.com`. Terraform formatting and validation pass.
The administrator SSO session expired during the preflight inventory; renew it
before producing the saved admin-edge plan.

The first admin-edge plan had SHA-256
`ecbd3280923cb1229e7afc91a8fec532359bcf0d581926117bd9c2c2b943c2cf`
and must not be applied. Review found two avoidable problems: the shared policy
data source made Terraform propose an unknown-value rewrite of the otherwise
unchanged media bucket policy, and distribution-wide `403/404 -> index.html`
responses could have converted API authorization/not-found responses into HTML
status 200. The admin uses hash navigation, so SPA error rewriting is not
needed. Terraform now isolates the CloudFront grant to the admin-assets policy
and has no custom error responses. Formatting and validation pass; regenerate
the saved plan under a new filename and verify that only the admin bucket
policy changes.

The corrected `staging-admin-edge-v2.tfplan` had SHA-256
`b49256b23d710ac2700bd57730129053dc0c7af0803267ad34d5e1f87de02c4b`.
Its saved-plan JSON showed three CloudFront resources created, only the
admin-assets bucket policy updated, no media policy change, no custom error
responses, and no destroys. The user applied it successfully: CloudFront
distribution `E1LIGJBBJV4A65` is deployed at
`db4g862zbz6gf.cloudfront.net`, with the
`admin.staging.knitnprint.com` alias. The local admin build passed with
`VITE_STOREFRONT_URL=https://staging.knitnprint.com`; its six output files
include the noindex HTML and robots.txt. An S3 sync dry run proposed exactly
those six files for the admin-assets bucket. Next, upload that build,
verify the CloudFront-served site and API behavior, then publish the Namecheap
`admin.staging` CNAME.

The user uploaded all six admin build files to the private S3 bucket with
`Cache-Control: no-cache`. Read-only pre-DNS HTTPS checks through CloudFront,
preserving the `admin.staging.knitnprint.com` host, returned admin HTML `200`,
the JavaScript asset `200`, `/robots.txt` `200` with `Disallow: /`, API health
`200` JSON, and unauthenticated `/api/admin/auth/me` `401` JSON. CloudFront
applied `X-Robots-Tag: noindex, nofollow, noarchive, nosnippet` to static
responses. The API `401` was not rewritten to HTML. Namecheap authoritative
DNS currently has no `admin.staging.knitnprint.com` CNAME; publish host
`admin.staging` pointing to `db4g862zbz6gf.cloudfront.net` next, then verify
the public hostname and test admin sign-in interactively.

The user added the `admin.staging` CNAME. Namecheap authoritative DNS and
public resolvers return the intended CloudFront target, and the admin page is
now browser-accessible. Initial owner credentials have not been created in the
recorded workflow. A minimal one-time owner bootstrap is prepared locally:
Terraform creates an empty temporary Secrets Manager secret and an ECS task
definition using the existing API image's `create_owner` binary. The existing
one-off bootstrap execution role gains read access to that secret; the normal
application execution role does not. The secret's `email`, `name`, and
`password` values will be entered by the operator in the AWS console after
Terraform applies, not written to Terraform or this guide. Formatting and
validation pass. Review a saved plan before applying; then run the task once,
verify login, and remove the temporary secret/task definition through a
reviewed Terraform cleanup.

The saved `staging-owner-bootstrap.tfplan` has SHA-256
`5075d1b718856decde6e3849fac2bb0e361593bdf2c5d40e4ddbbded8b52196d`.
Text and JSON review show exactly two creates (temporary secret and Fargate
owner task definition), two in-place one-off execution-role updates, and no
destroys or normal ECS service change. The task runs the pinned API image's
`create_owner` binary, injects the runtime database URL plus `email`, `name`,
and `password` JSON fields from Secrets Manager, and writes logs to the
existing API log group. The operator prefers to populate the secret through
the AWS console after applying the reviewed plan; no password is in Terraform
state or local files.

The user applied that exact owner-bootstrap plan successfully: two resources
added, two changed in place, none destroyed. The temporary secret ARN ends in
`owner-ODrDBx`, and task definition revision 1 is registered. A read-only
Secrets Manager metadata check found no version or `AWSCURRENT` label yet;
the owner credential has not been entered. Do not run the task until the
operator saves `email`, `name`, and `password` in the console and metadata
confirms a current version. Never read or print the secret value during checks.

The operator populated the temporary owner secret in the AWS console. A
metadata-only check now confirms one `AWSCURRENT` version; the value was not
retrieved. ECS task definition `knitnprint-staging-owner-bootstrap:1` is
active, runs the pinned API image's `/usr/local/bin/create_owner`, and injects
`DATABASE_URL`, `OWNER_EMAIL`, `OWNER_PASSWORD`, and `OWNER_NAME` via secret
references. RDS is `available`. The one-time task can now be run in a staging
public subnet with the application security group and Fargate 1.4.0; wait for
it to stop and inspect the exit code before testing login. Do not launch it
twice merely because the first command returns before task completion.

The user launched exactly one owner-bootstrap task,
`f4ef5e66039b4c13ad6d8787471117b8`, with no ECS launch failures. It
reached `STOPPED` with container exit code 0, and its isolated CloudWatch log
stream contains only `owner created`. No credential values were retrieved or
printed. The owner should now test interactive admin sign-in. After successful
login, remove the temporary owner task definition, secret, and its permission
from the one-off execution role through a reviewed Terraform cleanup plan.

## Safety constraints

- Do not use root for ordinary deployment or auditing.
- Do not create IAM-user access keys or permanent SES credentials.
- Do not delete historical IAM or SES resources merely because they look unused.
- Do not modify the `AWSSSO_..._DO_NOT_DELETE` provider or Identity Center-managed roles.
- Do not commit AWS credentials, cached SSO tokens, Terraform state, database passwords, Stripe secrets, or OAuth secrets.
- Do not run `terraform apply` until its saved plan and expected AWS changes have been reviewed.
- Keep staging resources in `eu-west-1` unless the architecture explicitly requires another Region, such as the CloudFront ACM certificate in `us-east-1`.
