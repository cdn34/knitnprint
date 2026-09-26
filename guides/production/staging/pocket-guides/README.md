# Staging deployment pocket guides

Short operator checklists for routine staging releases. The detailed runbooks
remain the source of truth for explanations, troubleshooting, rollback, and
unusual changes.

- `database-migration.md` — migration-first backend release
- `backend.md` — API release with no schema changes
- `storefront.md` — server-rendered storefront container
- `admin.md` — static S3 and CloudFront release

Print each guide separately using A4 or Letter portrait, 90–100% scale. Keep
code blocks unwrapped when the print dialog allows it.

Rules for every guide:

- Run commands from the repository root.
- Stop when a command or verification gate fails.
- Review saved Terraform plans before applying them.
- Never commit `.tfplan` files or credentials.
- Do not substitute production resources for the staging names shown here.
