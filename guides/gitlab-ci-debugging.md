# Debugging GitLab CI with `glab`

Use this guide to find a merge request's pipeline, identify failed jobs, save
their logs, and reproduce the failure locally. It complements the
[GitHub Actions debugging guide](ci-debugging.md).

This repository currently uses GitHub Actions and has no `.gitlab-ci.yml`.
The GitLab commands below are reference examples for a GitLab-hosted project;
they do not imply this repository has moved to GitLab. Run them inside that
project's clone, or select the project explicitly with `--repo`.

## 1. Install and authenticate

On macOS, install the CLI with Homebrew if needed:

```bash
brew install glab
glab --version
glab auth login --hostname gitlab.com
glab auth status
```

For a self-managed instance, authenticate against its hostname instead:

```bash
glab auth login --hostname gitlab.example.com
```

Use the interactive login rather than putting a token in a command or document.
Environment tokens such as `GITLAB_TOKEN` override stored credentials; an old
environment token can explain an authentication failure after a successful
login. Sourcing a shell configuration can update `PATH`, but does not sign you
in. See [authentication](https://docs.gitlab.com/cli/auth/login/).

Check which project your checkout selects:

```bash
git remote -v
glab api projects/:id | jq '{id, path_with_namespace, web_url}'
```

The API examples use `jq` to select JSON fields. Install it if unavailable.
In a clone whose remote points to GitHub, specify a real GitLab project:

```bash
glab ci list --repo gitlab.com/my-group/my-project
```

For API commands, `:id` is a `glab` placeholder for the selected project's ID.
It is not a literal GitLab project name. The CLI handles this substitution;
plain `curl` does not. See [API placeholders](https://docs.gitlab.com/cli/api/).

## 2. Understand the IDs before fetching logs

| Value | Identifies | Example |
| --- | --- | --- |
| MR IID | A merge request within one project | `!42` |
| Pipeline ID | One pipeline execution | `12345` |
| Job ID | One execution of a job within a pipeline | `224356863` |
| Commit SHA | The revision that the pipeline tested | Output of `git rev-parse HEAD` |

The numbers in this guide are examples. Replace them with IDs returned by your
GitLab project. A pipeline ID and a job ID are not interchangeable:

```bash
# Pipeline details.
glab ci get --pipeline-id 12345 --with-job-details

# Logs for one job execution.
glab ci trace 224356863
```

You can also find the IDs in GitLab's URLs:

```text
https://gitlab.com/my-group/my-project/-/merge_requests/42
https://gitlab.com/my-group/my-project/-/pipelines/12345
https://gitlab.com/my-group/my-project/-/jobs/224356863
```

## 3. Find the correct pipeline

Start with recent runs, then narrow by branch or failure status:

```bash
glab ci list --per-page 10
glab ci list --ref "$(git branch --show-current)" --per-page 10
glab ci list --status failed --per-page 10
glab ci list --source merge_request_event --per-page 10

# Look for pipelines associated with a particular local commit.
glab ci list --sha "$(git rev-parse HEAD)"
```

The latest branch pipeline is not necessarily the pipeline shown on an MR.
For example, branch pushes and MR events can create separate pipelines. Target
the MR directly when investigating a merge-blocking failure:

```bash
glab mr view 42
glab ci get --merge-request 42 --with-job-details
glab ci get --merge-request 42 --status failed --with-job-details

# Once selected, pin subsequent investigation to this pipeline ID.
glab ci get --pipeline-id 12345 --with-job-details --output json
```

Merged-result pipelines can test a generated merge commit rather than the
source branch's HEAD. Check the pipeline's SHA and source before assuming it
tested your local checkout. Local changes do not affect a remote pipeline until
they are committed, pushed, and included in a new run.

References: [listing pipelines](https://docs.gitlab.com/cli/ci/list/) and
[selecting an MR or pipeline](https://docs.gitlab.com/cli/ci/get/).

## 4. Watch status or inspect interactively

```bash
# Current branch: one status snapshot.
glab ci status

# Current branch: continuously update until the pipeline finishes.
glab ci status --live

# Wait for completion without the interactive live display.
glab ci status --wait

# Watch another branch.
glab ci status --branch main --live

# Navigate jobs and their logs in the terminal.
glab ci view
glab ci view --pipelineid 12345

# Open the selected pipeline in the browser.
glab ci view --pipelineid 12345 --web
```

`ci view` uses `--pipelineid`; `ci get` and `ci trace` use `--pipeline-id`.
Check `--help` if your installed CLI rejects an option from newer documentation.
In the interactive view, use arrow keys to select a job, Enter for logs, and Esc
to return. The view also exposes retry/run/cancel controls, which change CI state.

References: [live status](https://docs.gitlab.com/cli/ci/status/) and
[interactive pipeline view](https://docs.gitlab.com/cli/ci/view/).

## 5. Read and save a job's logs

GitLab calls the console output a job's **trace**. Unlike a status view,
`ci trace` prints the log itself and follows updates while the job runs:

```bash
# Choose a job interactively.
glab ci trace

# Select an exact execution by job ID.
glab ci trace 224356863

# Select by job name, scoped to a known pipeline.
glab ci trace frontend --pipeline-id 12345

# Save output locally.
glab ci trace 224356863 > /tmp/gitlab-ci-job-224356863.log

# Display and save output together.
glab ci trace 224356863 | tee /tmp/gitlab-ci-job-224356863.log

rg -n -i \
  'error|failed|panic|timeout|permission denied|RUSTSEC-|vulnerab' \
  /tmp/gitlab-ci-job-224356863.log
```

A name such as `frontend` can appear in many pipelines, retries, or parallel
jobs. Prefer a job ID for reproducible investigation. Read the lines before the
final exit-code message: they usually contain the actual failure. A pipeline's
status alone is not a substitute for those logs. See
[job tracing](https://docs.gitlab.com/cli/ci/trace/).

## 6. Use the API for exact metadata and raw logs

Use `glab api` when you need repeatable output or a CLI command is missing from
your installed version. These requests use the CLI's existing authentication:

```bash
GITLAB_PIPELINE_ID=12345
GITLAB_JOB_ID=224356863

glab api "projects/:id/pipelines/$GITLAB_PIPELINE_ID" \
  | jq '{id, sha, ref, source, status, web_url}'

glab api --paginate \
  "projects/:id/pipelines/$GITLAB_PIPELINE_ID/jobs?per_page=100" \
  | jq -r '.[] | [.id, .stage, .name, .status] | @tsv'

glab api "projects/:id/jobs/$GITLAB_JOB_ID" \
  | jq '{id, name, status, failure_reason, allow_failure, web_url}'

# Fetch the raw log as it exists now; this does not follow future output.
glab api "projects/:id/jobs/$GITLAB_JOB_ID/trace" \
  > "/tmp/gitlab-ci-job-$GITLAB_JOB_ID.log"
```

Pagination matters: the first page might not contain the failed job. For an MR's
pipeline, obtain its pipeline and owning project together:

```bash
GITLAB_MR_IID=42
glab api "projects/:id/merge_requests/$GITLAB_MR_IID" \
  | jq '{iid, sha, head_pipeline}'
```

If `head_pipeline` is null, there is no head pipeline to inspect yet. For fork
MRs, use the pipeline's owning `project_id` and correct host when querying its
jobs; do not assume the target project owns every execution. References:
[jobs and traces](https://docs.gitlab.com/api/jobs/) and
[merge request metadata](https://docs.gitlab.com/api/merge_requests/).

## 7. Save logs from every failed job in a pipeline

This recipe creates a directory containing one log per failed job plus a TSV
manifest mapping job IDs to names. It queries completed failures at that moment;
it does not wait for a running pipeline to finish.

```bash
set -o pipefail
GITLAB_PIPELINE_ID=12345
GITLAB_LOG_DIR="/tmp/gitlab-pipeline-$GITLAB_PIPELINE_ID"
mkdir -p "$GITLAB_LOG_DIR"

glab api --paginate \
  "projects/:id/pipelines/$GITLAB_PIPELINE_ID/jobs?scope[]=failed&per_page=100" \
  | jq -r '.[] | [.id, .name] | @tsv' \
  > "$GITLAB_LOG_DIR/failed-jobs.tsv"

while IFS=$'\t' read -r job_id job_name; do
  printf 'Fetching job %s (%s)\n' "$job_id" "$job_name"
  glab api "projects/:id/jobs/$job_id/trace" \
    > "$GITLAB_LOG_DIR/job-$job_id.log" || break
done < "$GITLAB_LOG_DIR/failed-jobs.tsv"

rg -n -i 'error|failed|panic|timeout|RUSTSEC-' "$GITLAB_LOG_DIR"
```

An empty manifest can mean there are no failed jobs yet. If a pipeline delegates
work to a child/downstream pipeline, inspect that pipeline's jobs as well; the
parent's job list alone may not contain the failing test execution. The
interactive pipeline view can help follow those links.

## 8. Download artifacts from the correct execution

Logs are console text. Artifacts are files a job uploaded, such as test reports,
screenshots, Playwright traces, or coverage reports. They are available only if
the job was configured to publish them and they have not been erased or expired.

For a specific failed execution, download its archive by **job ID**:

```bash
GITLAB_JOB_ID=224356863
GITLAB_ARTIFACT_DIR="/tmp/gitlab-artifacts-$GITLAB_JOB_ID"
mkdir -p "$GITLAB_ARTIFACT_DIR"

glab api "projects/:id/jobs/$GITLAB_JOB_ID/artifacts" \
  > "$GITLAB_ARTIFACT_DIR/artifacts.zip"

unzip -l "$GITLAB_ARTIFACT_DIR/artifacts.zip"
unzip "$GITLAB_ARTIFACT_DIR/artifacts.zip" -d "$GITLAB_ARTIFACT_DIR/files"
```

For the latest **successful** pipeline on a ref, the convenience command is:

```bash
glab job artifact main frontend --path /tmp/gitlab-main-frontend-artifacts
glab job artifact refs/merge-requests/42/head frontend \
  --path /tmp/gitlab-mr42-frontend-artifacts
```

That convenience command can retrieve files from a different execution than the
failed one you are debugging. Use the job-ID API for exact provenance. A 404 can
also mean missing permissions or an incorrect project, rather than proof the
job never produced artifacts. References:
[artifact downloads](https://docs.gitlab.com/api/job_artifacts/) and
[`glab job artifact`](https://docs.gitlab.com/cli/job/artifact/).

## 9. Diagnose configuration and runner failures

Inspect `.gitlab-ci.yml`, any `include` files, and the failed job's setup before
rerunning commands. Linting can distinguish configuration errors from a test
failure:

```bash
# Run inside a GitLab project containing this configuration file.
glab ci lint .gitlab-ci.yml
glab ci lint --dry-run --ref "$(git branch --show-current)" --include-jobs
```

Lint and simulation validate configuration; they do not execute the tests. See
[CI lint](https://docs.gitlab.com/cli/ci/lint/).

| Symptom | Inspect first |
| --- | --- |
| Job stays pending | Matching runner tags, runner availability, protected-ref restrictions |
| Job is skipped or absent | `workflow: rules`, job `rules`, and pipeline source |
| Failure before the script | Image pull, service startup, checkout, runner/system error |
| Script/test failure | First error in the trace and the command's exit code |
| Works locally but fails in CI | Runtime/image versions, lockfiles, OS, services, environment |
| Database connection refused | Service alias, database credentials, readiness checks |
| Private project returns 401/403/404 | Host, selected project, token scope, project membership |

For container-based GitLab jobs using a PostgreSQL service, the database host
is typically the configured service alias, such as `postgres`, rather than
`localhost`. Local commands using a published Docker port use `localhost`.
Check the actual executor and service configuration; see
[PostgreSQL services](https://docs.gitlab.com/ci/services/postgres/).

For KnitNPrint's local database creation, migration, browser installation,
isolated server ports, and cleanup commands, follow the
[local reproduction section](ci-debugging.md#run-the-full-frontend-suite-in-isolation)
of the companion guide. Those steps are useful regardless of the remote CI
provider. Match the failing GitLab job's setup and command rather than assuming
GitHub and GitLab runners have identical environments.

## 10. Retry deliberately, then verify the result

After preserving the original failure, retry a job if the failure appears
transient:

```bash
# This starts another job execution and can incur runner usage or side effects.
glab ci retry 224356863

# Or select a named job within a known pipeline.
glab ci retry frontend --pipeline-id 12345
```

A retry reruns the existing pipeline's revision; it does not upload uncommitted
local fixes. After a code change, commit and push through the project's normal
workflow, then inspect the MR's new pipeline. Read its resulting job IDs and
logs instead of continuing to inspect the old failure. See
[retrying jobs](https://docs.gitlab.com/cli/ci/retry/).

## GitHub-to-GitLab command reference

| Task | GitHub CLI | GitLab CLI |
| --- | --- | --- |
| View PR/MR | `gh pr view 19` | `glab mr view 42` |
| List executions | `gh run list` | `glab ci list` |
| Inspect one execution | `gh run view RUN_ID` | `glab ci get --pipeline-id PIPELINE_ID` |
| Watch branch status | `gh pr checks --watch` | `glab ci status --live` |
| Inspect MR's failed jobs | `gh pr checks 19` | `glab ci get --merge-request 42 --status failed --with-job-details` |
| Read a job's log | `gh run view RUN_ID --job JOB_ID --log` | `glab ci trace JOB_ID` |
| Save a raw job log | Redirect `gh run view ... --log` | Redirect `glab api projects/:id/jobs/JOB_ID/trace` |

GitHub's `--log-failed` collects failed steps across a workflow run. GitLab's
`ci trace` targets one job; use the failed-jobs recipe above to collect multiple
jobs. Keep the selected project, pipeline ID, job IDs, and commit SHA with your
debugging notes so the evidence remains tied to the execution you investigated.
