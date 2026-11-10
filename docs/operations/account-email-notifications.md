# Account email delivery and manual recovery

Registration, verification resends, password recovery, and email-change requests
now create `notification_jobs` entries in the same transaction as their account
tokens. The API attempts delivery after that transaction commits. A failed send
leaves a pending job with a recorded attempt and an actionable error; it does not
roll back registration or disappear into service logs.

In **Email notifications**, search for the recipient or use the types
`email_verification`, `password_reset`, and `email_change`. Pending jobs with a
failed attempt remain visible under **Outstanding** or **Pending** while automatic
retries remain available. Exhausted retries use **Failed**. Authorized staff can
use **Send now**, give a reason, and inspect the outcome. The ordinary notification
worker also retries these jobs using the existing lease, backoff, and retry cap.

An account job contains a reference to its account-token record, never a plaintext
token, password, action URL, or email body. Every delivery attempt generates a new
link in memory and updates only its hash and expiry before sending. Previously
issued links for that request stop working. Successful jobs cannot be sent twice.
Public resend cooldowns are preserved; an authorized manual recovery does not
have to wait for that cooldown.

Retries recheck that the account is active, the recipient matches the current
request, and the token has not been used or replaced. Verification jobs for an
already activated account are suppressed. Password recovery and email-change jobs
are suppressed after their original request window expires. Completed, replaced,
and unavailable requests are also suppressed, with an explanation visible in the
administrator. Email-change retries always target the saved proposed address.

Apply migration `0061_account_email_notifications.sql` before deploying the API
and worker. It adds the account-token reference and the suppressed-attempt outcome.
Account-token retention cleanup redacts non-commercial outbox recipients while
preserving delivery-attempt history. Existing order confirmations keep their
current delivery behavior.

Earlier account emails were never persisted as notification jobs, so this
migration does not invent delivery history for old tokens or automatically resend
them. New requests and attempts are recorded from deployment onward.
