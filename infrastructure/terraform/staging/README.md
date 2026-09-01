# Staging Terraform

All staging stacks live below this directory and use the
`knitnprint-staging-*` resource prefix and `Environment=staging` tag.

Start with [`bootstrap`](./bootstrap). Later staging stacks must use the
bootstrap output as their remote-state bucket and distinct keys below
`staging/`, for example `staging/dns/terraform.tfstate` and
`staging/application/terraform.tfstate`.

Never point this configuration at production buckets, roles, secrets, or state.
