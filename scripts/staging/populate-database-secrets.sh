#!/usr/bin/env bash

# Keep generated passwords out of shell tracing, command arguments, files, and
# Terraform state. Secret values travel to AWS CLI only through standard input.
set +x
set -euo pipefail

profile_name="${AWS_PROFILE:-knitnprint-administrator}"
region_name="${AWS_REGION:-eu-west-1}"
expected_account="739863594156"
database_host="knitnprint-staging-postgres.cdugyku62w1w.eu-west-1.rds.amazonaws.com"
database_port="5432"
database_name="knitnprint"

for command_name in aws jq openssl; do
  command -v "${command_name}" >/dev/null || {
    echo "${command_name} is required" >&2
    exit 1
  }
done

actual_account="$(
  aws sts get-caller-identity \
    --profile "${profile_name}" \
    --region "${region_name}" \
    --query Account \
    --output text \
    --no-cli-pager
)"

if [[ "${actual_account}" != "${expected_account}" ]]; then
  echo "Refusing to write secrets in unexpected AWS account ${actual_account}." >&2
  exit 1
fi

put_database_secret() {
  local secret_name="$1"
  local database_role="$2"
  local database_password
  local database_url

  database_password="$(openssl rand -hex 32)"
  database_url="postgresql://${database_role}:${database_password}@${database_host}:${database_port}/${database_name}?sslmode=require"

  printf '%s\n%s\n%s\n' \
    "${database_role}" \
    "${database_password}" \
    "${database_url}" \
    | jq -Rn '{username: input, password: input, url: input}' \
    | aws secretsmanager put-secret-value \
        --profile "${profile_name}" \
        --region "${region_name}" \
        --secret-id "${secret_name}" \
        --secret-string file:///dev/stdin \
        --query '{ARN:ARN,VersionId:VersionId}' \
        --output json \
        --no-cli-pager \
        >/dev/null

  unset database_password database_url
}

put_database_secret \
  "knitnprint/staging/database/migration-url" \
  "knitnprint_migration"
put_database_secret \
  "knitnprint/staging/database/runtime-url" \
  "knitnprint_runtime"

echo "Populated the staging migration and runtime database secrets without displaying their values."
