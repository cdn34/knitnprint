# Google sign-in

Set `GOOGLE_CLIENT_ID` on the API to a Google OAuth **Web application** client ID. Leave it unset to hide Google sign-in. No client secret is needed: the browser uses Google Identity Services and the backend verifies signed ID tokens. Keep the API under the storefront's `/api` origin.

For the staging Terraform application, set `google_client_id` (or `TF_VAR_google_client_id`) to populate this runtime setting. Its empty default keeps Google sign-in disabled. The client ID is public, not a secret.

In Google Cloud Console, configure the app branding, homepage and privacy-policy URLs, audience, and exact Authorized JavaScript origins for each environment. Use separate development/staging and production clients. For local testing register `http://localhost` and `http://localhost:3000` (or your chosen development port) and open the storefront using that hostname. Do not register the admin origin. This integration uses popup callbacks, so no redirect URI is required.

If your local database already applied Google sign-in as migration `0059`, follow the [local database recovery guide](../../guides/local-database-migration-recovery.md) before applying the rebased migration sequence. It includes a full rebuild and a data-preserving history repair. For a deployed staging or production database, use the [migration-conflict incident runbook](../../guides/production/migration-conflict-recovery.md); do not reset a live database whose data must be kept.

Apply database migrations before deploying the new API and frontend together. The schema adds optional passwords, Google identities, and temporary challenges; existing password accounts retain their credentials. Configure `GOOGLE_CLIENT_ID` only after deployment. Do not roll back to the old API once passwordless accounts exist: its credential query assumes passwords are non-null. Disabling Google only hides new Google authentication; existing Google-only users can use forgot password to establish a password.

If the storefront has a Content Security Policy, permit Google's GIS script, frame, and connection URLs described in Google's setup guide: https://developers.google.com/identity/gsi/web/guides/get-google-api-clientid . A restrictive Cross-Origin-Opener-Policy must allow popups (`same-origin-allow-popups`) where required. Do not weaken API JSON response security headers or origin checks.

Use the official button, and request identity only. No Google API access or refresh tokens are stored. Challenges expire after ten minutes, browser confirmation links must open in the initiating browser, and security reauthentication lasts five minutes. Customer retention cleanup removes expired challenges and cascades account deletion to identities and pending proofs. Google email mismatch blocks Google authentication; password/reset access to the original KnitNPrint email is the recovery route. If both are unavailable, contact support; there is no automatic email migration.

Before production activation, smoke-test with real Gmail, Workspace, and external-address Google accounts: new registration, existing-account linking, missing names, email confirmation, password setup/reset, both unlink directions, confirmed reconnection, email mismatch, blocked popup/cancellation, logout, and admin deletion. Verify desktop and mobile browser behavior. Automated tests use signed test fixtures or intercepted browser responses; production has no test-token bypass.

Google consent revocation is best effort from the customer's browser after local unlinking. Admin deletion removes local identity and sessions immediately; the customer can also remove app consent in their Google Account settings. A fresh deliberate Google signup after deletion creates an empty account, without recovering erased data.
