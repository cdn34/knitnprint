# Staging Terraform bootstrap stack

This stack creates prerequisites for staging only. It lives under
`infrastructure/terraform/staging` deliberately; production will have its own
root, state, credentials, and resource names.

It creates:

- one globally unique, private `knitnprint-staging-terraform-state-<account-id>`
  S3 state bucket in `eu-west-1`;
- versioning, S3-managed encryption, Block Public Access, and a TLS-only policy;
- native S3 state locking support for subsequent backends;
- a `$120` account-wide monthly budget with actual-spend alerts at 80% and 100%
  and a forecast alert at 100%.

The bootstrap apply uses the existing `knitnprint-administrator` SSO profile. It does not use root or permanent access keys.

## Access model

The sole human operator uses the temporary `knitnprint-administrator` Identity
Center profile for Terraform. Every mutation still uses a saved plan reviewed
immediately before apply, and no permanent access keys are used. ECS tasks and
other workloads receive separate least-privilege roles. A future GitHub OIDC
role will be limited to routine releases rather than general infrastructure.

## Initial local-state apply

The first apply necessarily uses local state because the state bucket does not exist yet. Never commit that state.

1. Copy `terraform.tfvars.example` to `terraform.tfvars` and replace the email placeholder.
2. Reauthenticate the `knitnprint-administrator` SSO profile if needed.
3. Initialize and validate the stack.
4. Save and review a plan.
5. Apply only the reviewed saved plan.

The initial bootstrap created one S3 bucket plus its protection resources, one
AWS Budget with three notifications, and a temporary restricted staging
permission set. The permission set was subsequently removed after the workflow
was simplified to one human administrator profile; it was never granted
application infrastructure permissions.

## Move bootstrap state to S3

After the first successful apply:

1. Confirm that `backend.tf` names
   `knitnprint-staging-terraform-state-739863594156`.
2. Run `terraform init -migrate-state` and approve the state copy only after
   confirming the source is local state and the destination is that bucket.
3. Confirm that a subsequent `terraform plan` is empty.

The resulting backend uses native S3 lockfiles through `use_lockfile = true`; no DynamoDB locking table is required.

The backend key is `staging/bootstrap/terraform.tfstate`. Production must use a
different bucket and a `production/...` key; it must never share staging state.

The current staging bootstrap completed this migration on 2026-09-02. A
post-migration plan reported no changes. The administrator profile is allowed
by the bucket policy and is used by every staging Terraform backend.
