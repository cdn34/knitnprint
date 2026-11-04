# Google sign-in implementation plan

## Agreed behavior

- Storefront customers can register and sign in with Google; staff authentication is unchanged.
- Link an existing registered account with the same normalized email, preserving customer ID, orders, addresses, and profile.
- Automatically link Gmail and verified Google Workspace addresses. Require current mailbox proof or the existing verified account password for other addresses.
- Identify Google accounts by their stable subject, and require matching Google and KnitNPrint emails on every Google authentication.
- Block email-change requests and confirmations while Google is connected. Changing email requires adding a password, unlinking Google, changing email, and reconnecting a matching Google account.
- Explicit Google unlinking disables automatic relinking; reconnection requires ownership confirmation.
- Never retain an unverified account's pre-existing password when Google/mailbox ownership establishes the real account owner.
- Collect missing required names before account creation. Do not merge guest purchases by email.

## Security and storage

- Nullable password hashes; unique Google subject and one Google identity per customer, cascading on account deletion.
- Official Google Identity Services button, no One Tap or automatic sign-in.
- Validate signed ID tokens, audience, issuer, expiry, nonce, subject, and verified email with cached Google signing keys.
- Short-lived single-use challenges bound to the initiating browser and purpose/session.
- Existing opaque session cookies; no stored Google access, refresh, or ID tokens.
- Rate limits, origin protection, no-store responses, safe audit events.

## Account management

- Profile sign-in methods panel: connect/disconnect Google; set/change/remove password.
- Fresh authentication (five minutes) before security changes. First password requires Google; existing password changes require the current password.
- Removing Google requires password proof; removing password requires Google proof. Never remove the last method, including concurrent requests.
- Method changes revoke sessions, replace the current session, and invalidate account-action tokens.
- Forgot password creates/replaces a password for any active account, including Google-only accounts, without removing Google; revoke all sessions.
- Security notifications for method, password, and email changes; best-effort Google consent revocation when unlinking in the browser.

## Deletion and rollout

- Extend admin deletion and retention cleanup to Google identities and pending challenges. No customer self-deletion in this change.
- Preserve existing commercial-record retention. Fresh deliberate signup after deletion does not restore history.
- Extend OpenAPI and regenerate the TypeScript client.
- English, Portuguese, and Spanish UI; accessible desktop/mobile flows.
- GOOGLE_CLIENT_ID per environment, disabled when absent. Document Console origins/branding and browser policies; real staging smoke test before activation.

## Verification

- New signup, trusted automatic linking, external-email proof, unverified-account protection, conflicts, preserved identity/data, confirmed reconnection.
- Password setup/reset/removal, last-method protection, concurrent changes, session revocation, email lock, mismatched Google email recovery.
- Invalid credentials/audience/nonce, expiry, replay, rate limiting, deletion races, retention cleanup.
- Backend tests, formatting/Clippy, frontend typecheck/build, generated API consistency, desktop/mobile browser coverage without production authentication bypasses.
