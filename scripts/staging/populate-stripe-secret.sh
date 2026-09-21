#!/usr/bin/env bash

# Prompt on the terminal so Stripe credentials never enter shell history,
# command arguments, files, Terraform configuration, plans, or state.
set +x
set -euo pipefail

profile_name="${AWS_PROFILE:-knitnprint-administrator}"
region_name="${AWS_REGION:-eu-west-1}"
expected_account="739863594156"
secret_name="knitnprint/staging/stripe"

for command_name in aws jq; do
  command -v "${command_name}" >/dev/null || {
    echo "${command_name} is required" >&2
    exit 1
  }
done

if [[ ! -t 0 ]]; then
  echo "Run this script from an interactive terminal." >&2
  exit 1
fi

actual_account="$(
  aws sts get-caller-identity \
    --profile "${profile_name}" \
    --region "${region_name}" \
    --query Account \
    --output text \
    --no-cli-pager
)"

if [[ "${actual_account}" != "${expected_account}" ]]; then
  echo "Refusing to write a secret in unexpected AWS account ${actual_account}." >&2
  exit 1
fi

read -r -s -p "Stripe test secret key (sk_test_...): " stripe_secret_key
echo
read -r -s -p "Stripe webhook signing secret (whsec_...): " stripe_webhook_secret
echo

if [[ "${stripe_secret_key}" != sk_test_* ]]; then
  echo "The staging Stripe key must start with sk_test_." >&2
  exit 1
fi

if [[ "${stripe_webhook_secret}" != whsec_* ]] || (( ${#stripe_webhook_secret} <= 6 )); then
  echo "The webhook signing secret must start with whsec_." >&2
  exit 1
fi

printf '%s\n%s\n' "${stripe_secret_key}" "${stripe_webhook_secret}" \
  | jq -Rn '{secret_key: input, webhook_secret: input}' \
  | aws secretsmanager put-secret-value \
      --profile "${profile_name}" \
      --region "${region_name}" \
      --secret-id "${secret_name}" \
      --secret-string file:///dev/stdin \
      --query '{ARN:ARN,VersionId:VersionId}' \
      --output json \
      --no-cli-pager \
      >/dev/null

unset stripe_secret_key stripe_webhook_secret
echo "Populated the staging Stripe secret without displaying its value."
