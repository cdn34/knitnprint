# Production Terraform

Production configuration will live below this directory. It is intentionally
not derived from staging state and must use:

- the `knitnprint-production-*` resource prefix;
- `Environment=production` tags;
- temporary human SSO administration for reviewed infrastructure changes and
  separate least-privilege workload/release roles;
- `knitnprint-production-terraform-state-<account-id>`;
- separate media, admin-assets, logging, database, secrets, and application
  resources.

No production Terraform stack has been created yet. Reusable infrastructure
should eventually be implemented as modules called independently by the two
environment roots, not by sharing state or Terraform workspaces.

Keep production authorization proportional to the actual workflow. Human
infrastructure changes may use a temporary administrator SSO session with
mandatory saved-plan review; routine GitHub deployments should use a small OIDC
role limited to image publication, ECS rollout, migrations, admin asset upload,
and CloudFront invalidation. Do not recreate a large action-by-action Terraform
policy merely to avoid clearly identified temporary administrator access.
