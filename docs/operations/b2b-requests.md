# Business requests

The storefront B2B form posts to `/api/b2b/requests`. A successful response is
returned only after the request, original attachment, image preview and support
notification job have been committed to PostgreSQL together. The request key
makes retries idempotent, including a retry after a lost response.

`b2b_requests` holds the complete original file as `bytea`, its SHA-256 checksum,
metadata and a preview for PNG, JPEG and SVG files. Include this table in normal
database backups. Originals do not depend on temporary files, browser state or
the object-storage cleanup process. Files are limited to 10 MiB; supported
formats are AI, EPS, PDF, SVG, PNG and JPEG. Uploads use the configured malware
scanner. SVG previews use image rendering and the API's sandboxed content policy.

In the admin, open **B2B** to list requests, review the saved image, download the
original, and track requests as New, In progress or Completed. Reading requests
and attachments requires `orders.read`; changing status requires `orders.fulfill`
and creates an audit record. Attachments are private and served with `no-store`.

Staff with `orders.fulfill` can also delete a request from its detail page after
an explicit confirmation. This permanently removes the request, original file,
preview and notification job in one transaction, and retains a minimal audit
record of the action. Pending email jobs are removed; an actively sending job
must finish before deletion is allowed. Already sent emails are unaffected.

The API automatically delivers queued B2B notifications every ten seconds to
`support@knitnprint.com`, using the existing email configuration. Failed attempts
are retried with backoff, up to eight attempts; the admin shows delivery status.
Requests and files remain saved even when email delivery fails. The existing
`deliver_notifications` command also processes due B2B notifications. Development
mailbox mode captures messages locally; `EMAIL_DELIVERY=ses` sends through SES.
Any recipient allowlist must include `support@knitnprint.com`.

Local Vite proxies and storefront SSR accept `API_BASE_URL` when the API uses a
port other than 8080. Set the same URL for storefront and admin, for example
`API_BASE_URL=http://127.0.0.1:8082`.
