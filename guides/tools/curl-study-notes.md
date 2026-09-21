# cURL study notes

Last updated: 2026-08-30

These notes cover cURL fundamentals, option discovery, common options by purpose, practical HTTP examples, and the safe role cURL can play in Google OAuth.

This workstation currently has cURL `8.5.0` for Linux AMD64. Run `curl --version` before relying on an option because available features depend on the installed build.

Official references:

- [cURL command-line manual](https://curl.se/docs/manpage.html)
- [Google OAuth 2.0 overview](https://developers.google.com/identity/protocols/oauth2)
- [Google OAuth for desktop applications](https://developers.google.com/identity/protocols/oauth2/native-app)
- [Google OpenID Connect reference](https://developers.google.com/identity/openid-connect/openid-connect)
- [Google OAuth for limited-input devices](https://developers.google.com/identity/protocols/oauth2/limited-input-device)

The complete short option inventory for the installed version is recorded in [curl-options-8.5.0.txt](./curl-options-8.5.0.txt). The online manual may describe options newer than the locally installed cURL.

## Mental model

A cURL command normally consists of:

```text
curl [how to connect] [what request to send] [how to handle the response] URL
```

For example:

```bash
curl --fail-with-body --show-error --location \
  --connect-timeout 5 --max-time 30 \
  --header 'Accept: application/json' \
  'https://api.example.com/products'
```

This command:

- treats HTTP 400 and 500 responses as command failures while preserving their bodies;
- prints useful errors;
- follows redirects;
- limits connection setup to five seconds and the whole operation to 30 seconds;
- asks the server for JSON.

Shell quoting matters. Put URLs and values containing `?`, `&`, spaces, `*`, `{}`, or `[]` inside single quotes unless shell expansion is intentional.

## Discovering every available option

The authoritative option list for the installed binary is produced by the binary itself:

```bash
curl --version
curl --help all
curl --manual
man curl
```

List help categories:

```bash
curl --help category
```

Inspect one category or option:

```bash
curl --help http
curl --help auth
curl --help output
curl --help verbose
curl --help data-urlencode
```

The installed build exposes these categories:

| Category | Use it for |
| --- | --- |
| `auth` | Basic, Digest, bearer, Kerberos, NTLM, and proxy authentication |
| `connection` | Interfaces, sockets, keepalive, address families, and connection routing |
| `curl` | cURL program behavior, configuration, parallel work, and help |
| `dns` | DNS servers, DNS-over-HTTPS, and address selection |
| `http` | HTTP methods, headers, cookies, redirects, compression, and versions |
| `output` | Download names, overwrite protection, and remote timestamps |
| `post` | Form, URL-encoded, raw, binary, and JSON request bodies |
| `proxy` | HTTP, HTTPS, and SOCKS proxies and their TLS/authentication settings |
| `tls` | Certificate authorities, client certificates, TLS versions, and cipher settings |
| `upload` | Upload files, append data, resume, and create remote directories |
| `verbose` | Headers, traces, timings, status output, and diagnostics |
| Protocol categories | `file`, `ftp`, `imap`, `pop3`, `scp`, `sftp`, `smtp`, `ssh`, `telnet`, and `tftp` |

## High-value options

### Request method and data

| Option | Meaning | Typical use |
| --- | --- | --- |
| `-X, --request METHOD` | Select an explicit method | `PUT`, `PATCH`, or `DELETE` when cURL cannot infer it |
| `-G, --get` | Put `--data*` fields into the URL query | Safely construct a GET query |
| `-d, --data VALUE` | Send form-style request data | Simple `application/x-www-form-urlencoded` POST |
| `--data-urlencode VALUE` | URL-encode a field | OAuth forms and values containing spaces or punctuation |
| `--data-binary DATA` | Preserve bytes and newlines | Send an exact file or payload |
| `--data-raw DATA` | Like `--data`, without treating `@` specially | Literal data beginning with `@` |
| `--json DATA` | Send JSON and set JSON headers | JSON APIs; supports literal JSON or `@filename` |
| `-F, --form FIELD` | Send multipart form data | File uploads mixed with ordinary fields |
| `-T, --upload-file FILE` | Upload a file as the request body | PUT, SFTP, FTP, or WebDAV-style uploads |
| `--url-query VALUE` | Add an encoded query component | Build query strings without manually joining fields |

Avoid `-X POST` when `--data`, `--json`, or `--form` already selects POST. Use `--request` only when the method genuinely needs to be overridden.

### Headers, identity, authentication, and state

| Option | Meaning | Typical use |
| --- | --- | --- |
| `-H, --header HEADER` | Add, replace, or remove a header | Content negotiation, bearer tokens, custom API headers |
| `-A, --user-agent VALUE` | Set `User-Agent` | Identify a script to an API |
| `-e, --referer URL` | Set `Referer` | Rare compatibility testing |
| `-u, --user USER:PASSWORD` | Supply HTTP/server credentials | Basic or Digest authentication |
| `--basic`, `--digest`, `--anyauth` | Choose an HTTP auth scheme | Match the server's supported authentication |
| `-n, --netrc` | Read credentials from `.netrc` | Avoid typing reusable credentials in commands |
| `--netrc-file FILE` | Use a specific netrc file | Isolate credentials for one integration |
| `-b, --cookie DATA_OR_FILE` | Send cookies | Continue an existing HTTP session |
| `-c, --cookie-jar FILE` | Save response cookies | Preserve a test session between commands |

Use an `Authorization: Bearer` header for OAuth access tokens. Never put a bearer token in the URL because URLs are commonly logged.

`--location-trusted` forwards credentials across redirects to different hosts. It is intentionally dangerous and should almost never replace ordinary `--location`, which protects authentication and cookie headers on cross-origin redirects.

### Status, errors, and diagnostics

| Option | Meaning | Typical use |
| --- | --- | --- |
| `-f, --fail` | Return nonzero for HTTP errors and suppress their bodies | Downloads and scripts that do not need error JSON |
| `--fail-with-body` | Return nonzero but keep the error body | APIs that explain errors in JSON |
| `-S, --show-error` | Show errors when silent mode is active | Usually combine with `--silent` |
| `-s, --silent` | Hide progress and errors | Machine-readable output; usually use `-sS` |
| `--no-progress-meter` | Hide only the progress meter | Keep warnings and errors visible |
| `-i, --include` | Include response headers before the body | Quick interactive inspection |
| `-I, --head` | Send HEAD and show headers | Inspect metadata without downloading a body |
| `-D, --dump-header FILE` | Save response headers separately | Examine cookies, caching, or rate limits |
| `-v, --verbose` | Show connection and protocol details | Diagnose DNS, TLS, redirects, and headers |
| `--trace FILE` | Record a detailed binary-aware trace | Deep protocol diagnosis |
| `--trace-ascii FILE` | Record a readable trace | Deep diagnosis when payloads are textual |
| `--trace-time` | Add timestamps to trace output | Identify delays |
| `-w, --write-out FORMAT` | Print response metadata | Status codes and timing measurements |

Verbose and trace output can contain authorization headers, cookies, request bodies, and personal data. Redact it before sharing and do not commit traces.

### Redirects, timing, retries, and limits

| Option | Meaning | Typical use |
| --- | --- | --- |
| `-L, --location` | Follow redirects | Downloads and endpoints that redirect safely |
| `--max-redirs N` | Limit redirect count | Prevent redirect loops |
| `--connect-timeout SECONDS` | Limit connection establishment | Fail quickly on unreachable services |
| `-m, --max-time SECONDS` | Limit the complete operation | Bound automation runtime |
| `--retry N` | Retry transient failures | Unreliable networks and idempotent requests |
| `--retry-delay SECONDS` | Set delay between retries | Respect a service's recovery time |
| `--retry-max-time SECONDS` | Limit total retry period | Bound retrying automation |
| `--retry-connrefused` | Retry connection refusal | Services that are still starting |
| `--retry-all-errors` | Retry every cURL error | Only when repeating the operation is safe |
| `--limit-rate RATE` | Limit transfer bandwidth | Avoid saturating a connection |
| `--max-filesize BYTES` | Reject an advertised oversized download | Basic download guardrail |
| `-Y, --speed-limit RATE` | Define an unacceptable transfer rate | Abort stalled transfers |
| `-y, --speed-time SECONDS` | Time allowed below the speed limit | Abort stalled transfers |
| `--rate RATE` | Limit request frequency | Respect API rate limits |

Do not blindly retry non-idempotent operations such as order creation or payment submission. A retry is safe only when the API supplies an idempotency mechanism or the operation itself is idempotent.

### Response output and downloads

| Option | Meaning | Typical use |
| --- | --- | --- |
| `-o, --output FILE` | Save to a chosen filename | API responses or downloads |
| `-O, --remote-name` | Use the final URL path's filename | Ordinary downloads |
| `-J, --remote-header-name` | Use `Content-Disposition` filename | Server-selected download names; combine carefully with `-O` |
| `--output-dir DIR` | Choose the download directory | Keep artifacts organized |
| `--create-dirs` | Create directories needed by `--output` | Nested output paths |
| `--no-clobber` | Do not overwrite existing files | Protect previous downloads |
| `--remove-on-error` | Delete incomplete output after failure | Avoid mistaking partial files for valid files |
| `-C, --continue-at OFFSET` | Resume a transfer; `-` discovers offset | Large interrupted downloads |
| `-r, --range RANGE` | Request selected bytes | Partial downloads and media inspection |
| `-R, --remote-time` | Preserve the remote modification time | Mirrors and archives |
| `-z, --time-cond TIME_OR_FILE` | Transfer only if modified | Lightweight conditional downloads |
| `--etag-save FILE` | Save a response ETag | Cache-aware downloads |
| `--etag-compare FILE` | Send a stored ETag | Skip unchanged content |

### TLS and connection routing

| Option | Meaning | Typical use |
| --- | --- | --- |
| `--cacert FILE` | Use a specific CA bundle | Private enterprise certificate authorities |
| `--capath DIR` | Use a directory of CA certificates | Managed CA directories |
| `-E, --cert CERT` | Present a client certificate | Mutual TLS |
| `--key FILE` | Supply the client private key | Mutual TLS with separate key material |
| `--cert-type TYPE`, `--key-type TYPE` | Select certificate/key format | PEM, DER, or PKCS#12 integrations |
| `--cert-status` | Require valid OCSP stapling | Environments that mandate it |
| `--tlsv1.2`, `--tlsv1.3` | Set the minimum TLS version | Compatibility or security testing |
| `--tls-max VERSION` | Set the maximum TLS version | Diagnose version negotiation |
| `-4, --ipv4`, `-6, --ipv6` | Force an address family | Diagnose IPv4/IPv6 differences |
| `--interface NAME` | Bind to an interface/address | Multi-network testing |
| `--resolve HOST:PORT:ADDRESS` | Override DNS while preserving hostname/TLS | Test a new load balancer before DNS changes |
| `--connect-to FROM:TO` | Route a connection to another host/port | Backend routing tests |
| `--unix-socket PATH` | Connect through a Unix socket | Local Docker daemon or local service APIs |
| `-x, --proxy URL` | Use a proxy | Corporate or debugging proxies |
| `--noproxy HOSTS` | Bypass proxy for selected hosts | Local or internal services |
| `--socks5-hostname HOST:PORT` | Use SOCKS5 and resolve remotely | Privacy or bastion routing |

Never normalize `-k`/`--insecure` as a fix. It disables certificate verification and can conceal interception or a broken TLS configuration. Install the correct CA or correct the hostname instead.

### Transfer behavior and multiple requests

| Option | Meaning | Typical use |
| --- | --- | --- |
| `--compressed` | Request and automatically decompress content | Reduce API/download bandwidth |
| `-K, --config FILE` | Read options from a file | Reusable commands without huge shell lines |
| `-q, --disable` | Ignore the default `.curlrc` | Reproducible automation |
| `-:, --next` | Start a new option set | Different requests in one cURL process |
| `-Z, --parallel` | Perform multiple transfers concurrently | Download several independent files |
| `--parallel-max N` | Limit concurrent transfers | Protect local and remote resources |
| `-g, --globoff` | Disable cURL URL globbing | URLs containing literal braces or brackets |
| `--proto PROTOCOLS` | Restrict allowed protocols | Harden scripts that accept URLs |
| `--proto-redir PROTOCOLS` | Restrict redirect protocols | Prevent redirects to unexpected protocols |
| `--http1.1`, `--http2`, `--http3` | Request a protocol version | Compatibility and performance testing |
| `-N, --no-buffer` | Stream output immediately | Server-sent events and live logs |

## Practical examples

### Check a health endpoint in automation

```bash
curl --fail --show-error --silent \
  --connect-timeout 3 --max-time 10 \
  'https://api.example.com/health'
```

Use this for health checks where any HTTP error should fail the command.

### GET with safely encoded query parameters

```bash
curl --fail-with-body --show-error --get \
  --data-urlencode 'q=knitting & printing' \
  --data-urlencode 'page=2' \
  'https://api.example.com/search'
```

Use `--data-urlencode` instead of manually replacing spaces and punctuation.

### POST JSON

```bash
curl --fail-with-body --show-error \
  --json '{"name":"Blue scarf","quantity":2}' \
  'https://api.example.com/orders'
```

For a larger payload:

```bash
curl --fail-with-body --show-error \
  --json @request.json \
  'https://api.example.com/orders'
```

### PATCH or DELETE

```bash
curl --fail-with-body --show-error \
  --request PATCH \
  --json '{"status":"fulfilled"}' \
  'https://api.example.com/orders/123'

curl --fail-with-body --show-error \
  --request DELETE \
  'https://api.example.com/orders/123'
```

### Upload a file with metadata

```bash
curl --fail-with-body --show-error \
  --form 'title=Product photo' \
  --form 'file=@photo.jpg;type=image/jpeg' \
  'https://api.example.com/media'
```

### Inspect status, headers, and timing separately

```bash
curl --silent --show-error \
  --dump-header response-headers.txt \
  --output response-body.json \
  --write-out 'status=%{http_code} total=%{time_total}s\n' \
  'https://api.example.com/products'
```

### Download safely and resume if interrupted

```bash
curl --fail --show-error --location \
  --remote-name --remove-on-error \
  'https://downloads.example.com/archive.zip'

curl --fail --show-error --location \
  --continue-at - --remote-name \
  'https://downloads.example.com/archive.zip'
```

After downloading, verify a publisher-provided checksum:

```bash
sha256sum archive.zip
```

Compare that value with a checksum obtained through a trusted publisher channel.

### Test a server before changing DNS

```bash
curl --fail-with-body --show-error \
  --resolve 'staging.example.com:443:203.0.113.10' \
  'https://staging.example.com/health'
```

This connects to `203.0.113.10` while retaining `staging.example.com` for HTTP host routing and TLS verification.

### Preserve a cookie session during local testing

```bash
curl --fail-with-body --show-error \
  --cookie-jar cookies.txt \
  --json '{"email":"person@example.com","password":"example-only"}' \
  'https://local.example.test/api/login'

curl --fail-with-body --show-error \
  --cookie cookies.txt \
  'https://local.example.test/api/account'
```

Cookie jars contain authentication material. Use only test credentials, restrict file permissions, and delete the file after testing. Do not automate real Google login by copying browser cookies.

### Call an API with a bearer token

```bash
read -rsp 'Access token: ' curl_access_token
printf '\n'

curl --fail-with-body --show-error \
  --header "Authorization: Bearer ${curl_access_token}" \
  --header 'Accept: application/json' \
  'https://api.example.com/account'

unset curl_access_token
```

This keeps the token out of shell history, although command arguments may still be observable by other sufficiently privileged local processes while cURL is running. For production automation, use a proper secret store and an application client library.

### Retry a service that is starting

```bash
curl --fail --show-error --silent \
  --retry 10 --retry-connrefused --retry-delay 2 \
  --connect-timeout 2 --max-time 30 \
  'http://127.0.0.1:8080/health'
```

### Fetch independent URLs in parallel

```bash
curl --fail --show-error --parallel --parallel-max 4 \
  --remote-name \
  'https://downloads.example.com/one.zip' \
  'https://downloads.example.com/two.zip'
```

## Google sign-in and future requests

### Short answer

Yes, cURL can participate in a Google OAuth 2.0 flow:

1. Your application sends the user to Google's authorization page in the system browser.
2. The user signs in directly with Google and grants specific permissions.
3. Google returns a short-lived authorization code to your registered redirect URI.
4. cURL can exchange that code for a short-lived access token and, when offline access was requested, a refresh token.
5. Future API requests use the access token. When it expires, cURL can exchange the refresh token for a new access token.

cURL cannot safely replace the browser login page, and it is not itself an HTTP callback server. A desktop flow therefore needs a small loopback listener or an OAuth client library in addition to cURL.

The resulting values are tokens, not reusable Google account credentials:

- An **ID token** proves the authenticated identity to the OAuth client. Validate its signature, issuer, audience, expiry, and nonce before trusting it.
- An **access token** authorizes only the scopes and APIs that Google granted.
- A **refresh token** obtains new access tokens and must be protected like a password.
- None of these tokens reveals or replaces the user's Google password.
- A token issued to one OAuth client or API is not a general credential for unrelated websites that offer “Sign in with Google.”

### Recommended flow for WSL or a desktop CLI

Use Google's Authorization Code flow with PKCE and a loopback redirect:

```text
WSL/desktop application
  -> opens the system browser at accounts.google.com
  -> user authenticates and grants consent at Google
  -> Google redirects to http://127.0.0.1:<random-port>
  -> local listener validates state and extracts the code
  -> application exchanges code + PKCE verifier for tokens
```

The Google Cloud setup is:

1. Create or select a Google Cloud project.
2. Enable only the Google APIs the application needs.
3. Configure the OAuth consent screen and its audience.
4. Add the Google account as a test user while the app is in Testing status, where applicable.
5. Create an OAuth client with application type **Desktop app**.
6. Request the minimum scopes required.

For identity only, request `openid email profile`. For a Google API, add that API's narrowest suitable scope.

Generate a PKCE verifier, its SHA-256 challenge, and an anti-CSRF state value:

```bash
google_code_verifier="$(openssl rand -base64 64 | tr -d '=+/' | cut -c1-64)"
google_code_challenge="$(printf '%s' "$google_code_verifier" | openssl dgst -binary -sha256 | openssl base64 -A | tr '+/' '-_' | tr -d '=')"
google_oauth_state="$(openssl rand -hex 32)"
```

The browser authorization URL uses:

```text
https://accounts.google.com/o/oauth2/v2/auth
  ?client_id=YOUR_CLIENT_ID
  &redirect_uri=http://127.0.0.1:RANDOM_PORT
  &response_type=code
  &scope=openid%20email%20profile
  &access_type=offline
  &prompt=consent
  &state=YOUR_RANDOM_STATE
  &code_challenge=YOUR_PKCE_CHALLENGE
  &code_challenge_method=S256
```

The loopback listener must bind only to `127.0.0.1`, use the same redirect URI and port, compare returned `state` with the original value, reject missing or mismatched state, and accept the authorization code only once. Google's deprecated out-of-band copy/paste redirect must not be used.

After receiving and validating the authorization code, exchange it with cURL:

```bash
curl --fail-with-body --show-error \
  --data-urlencode "client_id=${GOOGLE_CLIENT_ID}" \
  --data-urlencode "code=${GOOGLE_AUTHORIZATION_CODE}" \
  --data-urlencode "code_verifier=${google_code_verifier}" \
  --data-urlencode "redirect_uri=http://127.0.0.1:${GOOGLE_OAUTH_PORT}" \
  --data-urlencode 'grant_type=authorization_code' \
  'https://oauth2.googleapis.com/token'
```

A web-server OAuth client also authenticates the token exchange according to its registered client type. A desktop client cannot keep a distributed client secret confidential, so do not treat such a secret as user authentication.

Google returns JSON similar to:

```json
{
  "access_token": "short-lived-secret",
  "expires_in": 3600,
  "refresh_token": "long-lived-secret-if-issued",
  "scope": "openid email profile",
  "token_type": "Bearer",
  "id_token": "signed-identity-jwt"
}
```

Never commit this response, print it in CI logs, or paste it into chat.

### Use and refresh a Google access token

Call the OpenID Connect user-info endpoint:

```bash
curl --fail-with-body --show-error \
  --header "Authorization: Bearer ${GOOGLE_ACCESS_TOKEN}" \
  'https://openidconnect.googleapis.com/v1/userinfo'
```

Refresh an expired access token:

```bash
curl --fail-with-body --show-error \
  --data-urlencode "client_id=${GOOGLE_CLIENT_ID}" \
  --data-urlencode "client_secret=${GOOGLE_CLIENT_SECRET}" \
  --data-urlencode "refresh_token=${GOOGLE_REFRESH_TOKEN}" \
  --data-urlencode 'grant_type=refresh_token' \
  'https://oauth2.googleapis.com/token'
```

Whether `client_secret` is required depends on the OAuth client type. Follow the current Google documentation for the registered client rather than inventing or omitting parameters.

Google access tokens expire. Refresh tokens can also expire or be revoked. In particular, an external OAuth consent screen in **Testing** status generally issues refresh tokens that expire after seven days when scopes beyond basic identity are requested. Code must handle `invalid_grant` by starting a new interactive authorization flow.

### Device authorization flow

Google also documents a browser-completion flow that is especially convenient with cURL, but it is only for OAuth clients registered as **TVs and Limited Input devices**. It should not be selected merely to bypass the desktop loopback flow.

Request a device code:

```bash
curl --fail-with-body --show-error \
  --data-urlencode "client_id=${GOOGLE_CLIENT_ID}" \
  --data-urlencode 'scope=openid email profile' \
  'https://oauth2.googleapis.com/device/code'
```

The response contains a `verification_url`, `user_code`, `device_code`, expiry, and polling interval. Open the verification URL in a browser, enter the user code, and consent. Poll no faster than the returned interval:

```bash
curl --fail-with-body --show-error \
  --data-urlencode "client_id=${GOOGLE_CLIENT_ID}" \
  --data-urlencode "client_secret=${GOOGLE_CLIENT_SECRET}" \
  --data-urlencode "device_code=${GOOGLE_DEVICE_CODE}" \
  --data-urlencode 'grant_type=urn:ietf:params:oauth:grant-type:device_code' \
  'https://oauth2.googleapis.com/token'
```

An `authorization_pending` response means the user has not finished. `slow_down` means increase the polling interval. Stop on denial or expiry rather than polling forever.

### Revoke access

When a token is no longer needed, revoke it:

```bash
curl --fail-with-body --show-error \
  --data-urlencode "token=${GOOGLE_REFRESH_TOKEN}" \
  'https://oauth2.googleapis.com/revoke'
```

Then remove the local stored token. Revoking a grant may invalidate other tokens issued for the same project and user, so understand the scope of revocation before doing it.

### OAuth security rules

- Never ask cURL to submit a Google username, password, MFA code, or recovery code.
- Never scrape Google's login HTML or copy Google browser cookies for automation.
- Use the system browser so the user can verify the real Google origin and TLS connection.
- Register the correct OAuth client type and exact redirect method.
- Use Authorization Code with PKCE for a desktop application.
- Generate and validate a cryptographically random `state`; use `nonce` when required for ID-token replay protection.
- Request the fewest scopes possible and verify the scopes actually granted.
- Store refresh tokens in an operating-system credential store, encrypted application secret store, or managed secrets service—not in Git, `.env` files committed to Git, shell history, or logs.
- Use the access token in an `Authorization` header, not a query string.
- Treat token endpoint JSON and verbose cURL traces as secrets.
- Handle expiry, revocation, `invalid_grant`, user denial, and partial scope grants.
- Use a Google client library for a real application unless there is a strong reason to implement the protocol directly.

## Sensible defaults for scripts

For read-only or otherwise safely repeatable HTTP operations, this is a useful starting point:

```bash
curl --disable \
  --fail-with-body --show-error --silent \
  --location --max-redirs 5 \
  --connect-timeout 5 --max-time 30 \
  --retry 3 --retry-delay 1 \
  --proto '=https' --proto-redir '=https' \
  'https://api.example.com/resource'
```

Adjust deliberately:

- Remove retries for unsafe non-idempotent requests.
- Allow HTTP only for an intentional local-development endpoint.
- Increase time limits for known long operations.
- Add authentication without exposing secrets in history or logs.
- Do not add `--insecure` to make TLS errors disappear.
