# Staging Terraform

All staging stacks live below this directory and use the
`knitnprint-staging-*` resource prefix and `Environment=staging` tag.

Start with [`bootstrap`](./bootstrap), followed by the external-DNS prerequisite
stack in [`dns`](./dns), then the single long-lived application root in
[`application`](./application). These roots use distinct state keys below
`staging/`: `staging/bootstrap/terraform.tfstate`,
`staging/dns/terraform.tfstate`, and
`staging/application/terraform.tfstate`. Files within `application` are merged
into one configuration; do not create a separate root for each AWS service.

All human-reviewed Terraform operations use the temporary
`knitnprint-administrator` SSO profile. A future GitHub OIDC role will handle
only routine releases; it will not have general infrastructure administration
access.

Never point this configuration at production buckets, roles, secrets, or state.
