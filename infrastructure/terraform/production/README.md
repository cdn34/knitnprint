# Production Terraform

Production configuration will live below this directory. It is intentionally
not derived from staging state and must use:

- the `knitnprint-production-*` resource prefix;
- `Environment=production` tags;
- a dedicated production deployer permission set and workload roles;
- `knitnprint-production-terraform-state-<account-id>`;
- separate media, admin-assets, logging, database, secrets, and application
  resources.

No production Terraform stack has been created yet. Reusable infrastructure
should eventually be implemented as modules called independently by the two
environment roots, not by sharing state or Terraform workspaces.
