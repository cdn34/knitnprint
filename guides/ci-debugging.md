# Debugging pull request CI

Use this guide to inspect GitHub Actions failures, reproduce them locally, and
clean up the disposable test environment. Run commands from the repository root.
The workflow definition is [ci.yml](../.github/workflows/ci.yml).

## Requirements

- GitHub CLI (`gh`), authenticated with access to this repository.
- Node.js 24 and the project's npm dependencies installed.
- Rust stable with Clippy and rustfmt.
- Docker running the local PostgreSQL container.
- `rg` for searching saved logs; Python 3 for the temporary configuration steps.

```bash
gh --version
gh auth login
gh auth status
node --version
docker ps --format '{{.Names}} {{.Ports}}'
```

If `gh` is installed but unavailable in the current shell, open a new terminal
or reload the shell configuration before retrying. Authentication failures need
`gh auth login`; sourcing a configuration file does not sign the CLI in.

## Inspect the PR and its live checks

The examples below use PR #19. Substitute the current PR number when needed.
Omit `19` to select the PR associated with the current branch.

```bash
gh pr view 19
gh pr checks 19

# Watch check status until the checks finish.
gh pr checks 19 --watch --interval 10

# Inspect structured results, including the commit being checked.
gh pr view 19 \
  --json number,url,headRefName,headRefOid,statusCheckRollup

# List recent runs for the current branch.
gh run list --branch "$(git branch --show-current)" --limit 3
```

`gh pr checks` returns a non-zero exit status when checks fail. This is expected
when investigating a failed PR. Compare the PR's `headRefOid` with local
`git rev-parse HEAD`: local fixes do not change GitHub's results until they are
committed, pushed, and checked by a new workflow run.

## Read GitHub Actions logs

Choose a run ID from `gh run list`. This command selects the latest run for the
current branch:

```bash
CI_RUN_ID=$(gh run list \
  --branch "$(git branch --show-current)" \
  --limit 1 --json databaseId --jq '.[0].databaseId')

gh run view "$CI_RUN_ID"
gh run view "$CI_RUN_ID" --json status,conclusion,jobs
gh run view "$CI_RUN_ID" --log-failed

# Keep a local copy of the failed steps.
gh run view "$CI_RUN_ID" --log-failed \
  > /tmp/knitnprint-ci-failed.log

rg -n \
  'RUSTSEC-|vulnerabilities found|toHaveAttribute|Resource not accessible|failed' \
  /tmp/knitnprint-ci-failed.log

# Download all job output instead of only failed steps.
gh run view "$CI_RUN_ID" --log > /tmp/knitnprint-ci-full.log

# Open the run in GitHub.
gh run view "$CI_RUN_ID" --web
```

Use the job IDs in the structured output to inspect a single job. For the
original failing PR #19 run, the exact commands were:

```bash
gh run view 39330885175 --log-failed

# Frontend job.
gh run view 39330885175 --job 118328466495 --log

# Security scan job.
gh run view 39330885175 --job 118328466093 --log
```

These IDs are historical examples, not aliases for the latest run. Log retrieval
may be unavailable while a job is still running; use `gh pr checks --watch` for
live status and download logs once the job completes. Downloaded logs remain
subject to the repository's Actions retention policy. Treat logs as potentially
sensitive even though GitHub masks configured secrets.

## Inspect the local checkout

```bash
git status --short
git log -5 --oneline
git diff --stat
git diff --check

cat .github/workflows/ci.yml
cat playwright.config.ts
cat backend/Cargo.toml

# Check whether a development server already occupies the default ports.
lsof -nP -iTCP:8080 -sTCP:LISTEN
lsof -nP -iTCP:3000 -sTCP:LISTEN
```

Playwright's default local configuration can reuse existing servers. Setting
`DATABASE_URL` on the test command does not reconfigure an already running API.
Use the isolated setup below when development servers are running, rather than
stopping them or allowing tests to modify development data.

## Reproduce the menu tests

Install the expected Chromium revision into a temporary directory:

```bash
PLAYWRIGHT_BROWSERS_PATH=/tmp/knitnprint-playwright-browsers \
npx playwright install chromium

PLAYWRIGHT_BROWSERS_PATH=/tmp/knitnprint-playwright-browsers \
npm run test:e2e -- --grep 'keeps.*menu' --workers=2
```

The second command uses the default server configuration. For isolation, create
the temporary configurations in the next section and add
`--grep 'keeps.*menu' --workers=2` to that section's `npx playwright test` command.

An enabled server-rendered button can be clicked before React attaches its
listeners. For controls requiring hydration, keep them disabled until they are
interactive. A menu toggle also changes its accessible name from `Open menu` to
`Close menu`: locate it by a stable attribute such as
`button[aria-controls="mobile-primary-nav"]`, then assert its accessible name
and `aria-expanded` state separately. Avoid arbitrary sleeps to hide the race.

## Run the full frontend suite in isolation

This setup uses a disposable database, API port **18080**, and storefront port
**13000**. The database name below is reserved for this debugging session. If
creation reports that it already exists, choose another disposable name and
update both database URLs below; do not drop an unfamiliar existing database.

Start PostgreSQL if it is not already running:

```bash
docker compose up -d --wait postgres
```

Check the container name with `docker ps`. The repository's default Compose
project uses `knitnprint-postgres-1`:

```bash
docker exec knitnprint-postgres-1 \
  createdb --username=knitnprint knitnprint_pr19_ci_7f3a

DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint_pr19_ci_7f3a \
npm run db:migrate
```

The frontend CI job migrates but does not seed the database. Use the same setup
when reproducing that job. Tests that require published catalog entries may
return early when the catalog is empty; a passing run does not exercise those
populated-catalog paths.

Create two temporary configuration files. Confirm these filenames do not
already contain work you want to keep before running the script:

```bash
python3 - <<'PY'
from pathlib import Path

source = Path('apps/storefront/vite.config.ts').read_text()
source = source.replace('127.0.0.1:8080', '127.0.0.1:18080')
source = source.replace('port: 3000', 'port: 13000')
Path('apps/storefront/vite.ci-local.config.ts').write_text(source)

Path('playwright.ci-local.config.ts').write_text("""
import { defineConfig } from '@playwright/test'
import config from './playwright.config'

export default defineConfig({
  ...config,
  use: { ...config.use, baseURL: 'http://127.0.0.1:13000' },
  webServer: [
    {
      command: 'cargo run -p knitnprint-api --bin knitnprint-api',
      url: 'http://127.0.0.1:18080/api/ready',
      env: {
        DATABASE_URL: 'postgres://knitnprint:knitnprint@localhost:5432/knitnprint_pr19_ci_7f3a',
        PORT: '18080',
        HOST: '127.0.0.1',
        WEB_ORIGINS: 'http://127.0.0.1:13000',
        STOREFRONT_BASE_URL: 'http://127.0.0.1:13000',
      },
      reuseExistingServer: false,
      timeout: 120_000,
    },
    {
      command: 'npm run dev --workspace=@knitnprint/storefront -- --config vite.ci-local.config.ts --port 13000 --host 127.0.0.1',
      url: 'http://127.0.0.1:13000',
      env: { API_BASE_URL: 'http://127.0.0.1:18080' },
      reuseExistingServer: false,
      timeout: 120_000,
    },
  ],
})
""")
PY

PLAYWRIGHT_BROWSERS_PATH=/tmp/knitnprint-playwright-browsers \
npx playwright test --config=playwright.ci-local.config.ts
```

All URL settings matter: Vite proxies browser API calls to port 18080,
`API_BASE_URL` directs server-rendered catalog requests there, and
`STOREFRONT_BASE_URL` makes verification/reset emails point back to port 13000.
The explicit `--port 13000` overrides the port in the npm development script.

The isolated run used to verify the PR #19 fixes completed with **28 passed**.
Playwright writes failure context and any configured traces under `test-results`:

```bash
rg --files -uuu test-results
```

## Verify dependency security fixes

Read the advisory IDs and patched versions from the audit logs before changing
dependencies. For PR #19, four advisories affected `rustls 0.23.45`; the targeted
fix was:

```bash
cargo update -p rustls --precise 0.23.46
git diff -- Cargo.lock
```

That command changes the lockfile. The version is the historical fix for this
incident, not a recommendation to downgrade a newer checkout.

To reproduce the audit locally, install `cargo-audit` normally or use the
official prebuilt binary. These are the exact macOS Apple Silicon download
commands used for this investigation; select a matching release asset on other
platforms:

```bash
gh release view cargo-audit/v0.22.2 \
  --repo rustsec/rustsec --json assets --jq '.assets[].name'

gh release download cargo-audit/v0.22.2 \
  --repo rustsec/rustsec \
  --pattern cargo-audit-aarch64-apple-darwin-v0.22.2.tgz \
  --dir /tmp/knitnprint-ci-tools

tar -xzf \
  /tmp/knitnprint-ci-tools/cargo-audit-aarch64-apple-darwin-v0.22.2.tgz \
  -C /tmp/knitnprint-ci-tools

/tmp/knitnprint-ci-tools/cargo-audit-aarch64-apple-darwin-v0.22.2/cargo-audit \
  audit \
  --ignore RUSTSEC-2023-0071 \
  --db /tmp/knitnprint-rustsec-db
```

The exception matches the existing CI configuration: SQLx keeps its disabled
optional MySQL/RSA dependency in the lockfile, while this application uses
PostgreSQL. Do not add exceptions to bypass newly reported vulnerabilities.

The `rustsec/audit-check` action also publishes a GitHub check. Its job needs
`contents: read` and `checks: write`; otherwise logs can include
`Resource not accessible by integration`. Fork PR tokens can remain read-only
despite configured permissions, so distinguish that limitation from an actual
vulnerability finding. See the action's
[permission documentation](https://github.com/rustsec/audit-check/tree/v2.0.0#granular-permissions).

## Run the remaining checks

```bash
npm run typecheck
npm run build

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p knitnprint-api --lib

git diff --check
```

The library command does not run every backend integration test. To reproduce
the broader backend CI test step, use the disposable database:

```bash
DATABASE_URL=postgres://knitnprint:knitnprint@localhost:5432/knitnprint_pr19_ci_7f3a \
cargo test --workspace
```

CI installs `citext` into `public` before running those integration tests. The
migrated disposable database above already has that extension. See the workflow
for the other jobs' setup, including SeaweedFS and owner creation for admin E2E.

Run builds and browser tests sequentially when they share a working tree: a
build can rewrite generated storefront files while the development server is
using them.

## Clean up the disposable environment

Once Playwright has exited, remove only the temporary configurations and the
database created for this session:

```bash
python3 - <<'PY'
from pathlib import Path

for name in [
    'playwright.ci-local.config.ts',
    'apps/storefront/vite.ci-local.config.ts',
]:
    Path(name).unlink(missing_ok=True)
PY

docker exec knitnprint-postgres-1 \
  dropdb --username=knitnprint knitnprint_pr19_ci_7f3a

git status --short
```

Do not drop the development `knitnprint` database or run `docker compose down -v`
as part of this cleanup. The temporary browser download and saved logs can stay
under `/tmp`; they are outside the repository and are not committed.
