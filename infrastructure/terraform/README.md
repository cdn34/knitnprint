# Terraform environments

Terraform configuration is separated by deployment environment so a plan or
state operation cannot accidentally mix staging and production.

```text
terraform/
|-- staging/
|   |-- bootstrap/
|   |-- dns/
|   `-- application/
`-- production/
```

Every deployable AWS resource uses the naming form:

```text
knitnprint-<environment>-<purpose>[-<account-id-or-region>]
```

Examples:

| Purpose | Staging | Production |
| --- | --- | --- |
| Media bucket | `knitnprint-staging-media-<account-id>` | `knitnprint-production-media-<account-id>` |
| Admin assets | `knitnprint-staging-admin-assets-<account-id>` | `knitnprint-production-admin-assets-<account-id>` |
| ALB logs | `knitnprint-staging-alb-logs-<account-id>` | `knitnprint-production-alb-logs-<account-id>` |
| API/ECR/IAM resources | `knitnprint-staging-api` | `knitnprint-production-api` |
| Terraform state | `knitnprint-staging-terraform-state-<account-id>` | `knitnprint-production-terraform-state-<account-id>` |

The `Environment` tag must match the name. Staging and production have separate
Terraform roots, backend buckets, state keys, workload roles, media buckets,
admin-assets buckets, databases, secrets, logs, and application resources.
Human operators use temporary Identity Center administrator sessions; workload
and automated release permissions remain environment-specific. Do not use
workspaces to combine these environments.

The application also validates `S3_BUCKET`: staging accepts only
`knitnprint-staging-*`, while production accepts only
`knitnprint-production-*`.
