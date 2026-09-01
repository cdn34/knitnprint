# Staging Terraform bootstrap stack

This stack creates prerequisites for staging only. It lives under
`infrastructure/terraform/staging` deliberately; production will have its own
root, state, credentials, and resource names.

It creates:

- one globally unique, private `knitnprint-staging-terraform-state-<account-id>`
  S3 state bucket in `eu-west-1`;
- versioning, S3-managed encryption, Block Public Access, and a TLS-only policy;
- native S3 state locking support for subsequent backends;
- a `$120` account-wide monthly budget with actual-spend alerts at 80% and 100% and a forecast alert at 100%;
- the `KnitNPrintStagingDeployer` IAM Identity Center permission set;
- assignment of that permission set to `knitnprint-administrators` in the management account.

The bootstrap apply uses the existing `knitnprint-administrator` SSO profile. It does not use root or permanent access keys.

## Deployer boundary

The deployer does not receive `AdministratorAccess` or `PowerUserAccess`. Its custom policy:

- permits the planned application services in `eu-west-1`;
- permits only ACM and WAF work in `us-east-1` for CloudFront;
- permits global CloudFront management;
- limits S3 management to `knitnprint-staging-*` buckets;
- limits IAM role and policy management and role passing to
  `knitnprint-staging-*` names;
- excludes IAM users and groups, Organizations, Account administration, IAM Identity Center administration, and changes to legacy `us-east-1` SES resources.

If a later Terraform plan exposes a missing action, add that specific action here and reapply the bootstrap stack with the administrator profile. Do not replace this boundary with administrator access merely to bypass an authorization failure.

## Initial local-state apply

The first apply necessarily uses local state because the state bucket does not exist yet. Never commit that state.

1. Copy `terraform.tfvars.example` to `terraform.tfvars` and replace the email placeholder.
2. Reauthenticate the `knitnprint-administrator` SSO profile if needed.
3. Initialize and validate the stack.
4. Save and review a plan.
5. Apply only the reviewed saved plan.

Expected resource changes are one S3 bucket plus its protection resources, one AWS Budget with three notifications, one Identity Center permission set and inline policy, and one group/account assignment.

## Move bootstrap state to S3

After the first successful apply:

1. Read the `state_bucket_name` output.
2. Copy `backend.tf.example` to `backend.tf`.
3. Replace `BUCKET_NAME` with the output value.
4. Run `terraform init -migrate-state` and approve the state copy only after confirming the bucket name.
5. Confirm that a subsequent `terraform plan` is empty.

The resulting backend uses native S3 lockfiles through `use_lockfile = true`; no DynamoDB locking table is required.

The backend key is `staging/bootstrap/terraform.tfstate`. Production must use a
different bucket and a `production/...` key; it must never share staging state.

## Configure the staging CLI profile

After the permission-set assignment finishes, configure a second local profile:

```bash
aws configure sso --profile knitnprint-staging
```

Reuse SSO session `knitnprint`, select `KnitNPrintStagingDeployer`, and use `eu-west-1` with JSON output. Subsequent Terraform stacks use this staging profile instead of the bootstrap administrator profile.
