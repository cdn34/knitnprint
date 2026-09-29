# Postmortem: staging signup emails denied by SES IAM policy

- Date: 2026-09-29
- Environment: staging
- Status: remediated; end-to-end inbox confirmation pending
- Affected workflow: customer signup email verification
- Detection: manual signup testing

## Summary

New staging customer accounts were created successfully, but their verification
emails were not delivered. The signup endpoint sends this email synchronously;
it does not rely on the scheduled notification worker. The application treated
delivery as best effort, logged the SES failure, and still returned a successful
account-creation response.

The staging API task role allowed `ses:SendEmail` only against the verified
sender identity. For the sandbox request, SES also evaluated the verified
destination identity as an authorization resource. The task role did not allow
that recipient resource, so SES returned `AccessDeniedException` and rejected
the send.

The live IAM policy now permits `ses:SendEmail` on all resources while retaining
conditions that restrict the sender to
`no-reply@staging.knitnprint.com` and the requested AWS Region to
`eu-west-1`. The transactional configuration set remains authorized in its own
statement. This removes the incorrect recipient-resource restriction without
allowing the application to use an arbitrary From address.

Staging remains in the SES sandbox. The application no longer has its own email
allowlist, but SES will still accept only recipients verified in the staging AWS
account until that account receives production access.

## Impact

- Customers could create staging accounts but did not receive the verification
  link required to complete email verification.
- Repeated signup attempts could not reuse the same email because the first
  attempt had already persisted the account.
- The failure potentially affected other staging email paths using the same API
  task role, although the incident was observed and diagnosed through signup
  verification emails.
- No customer or order data was lost. No database migration or service outage
  occurred.
- Production was not changed. Its SES domain and production-access work remains
  intentionally deferred until the production domain DNS is configured.

## Architecture and expected behavior

Signup verification is an inline application operation:

```text
POST signup
    |
    v
Persist customer and verification token
    |
    v
API task calls SES SendEmail immediately
    |
    +-- accepted: customer receives verification email
    |
    +-- rejected: warning is logged, but signup still succeeds
```

The notification worker is used for queued notifications such as order email;
it is not in the signup-verification path.

## Timeline

All timestamps are UTC.

1. Manual signup tests with SES-verified recipient addresses created accounts,
   but no verification messages arrived.
2. At `22:22:51`, the API logged `account email delivery failed`. The original
   error formatting exposed only `SES send failed: service error`, which was not
   sufficient to identify the AWS denial.
3. The SES account, verified identities, sender domain, configuration set,
   application environment, and ECS task configuration were inspected. They
   were present in `eu-west-1` and matched the staging configuration.
4. The SES IAM statements were split so the sender identity and configuration
   set were authorized independently. The account-email error log was also
   changed to preserve the underlying AWS SDK diagnostic. These changes were
   committed as `3f3f335`.
5. API image digest
   `sha256:52bcec34c6043a4c8d7eedd162bca91515e75a388644bca1bb2f0941e9d326aa`
   was pinned in `a2edc9c` and deployed as application task-definition revision
   `6`.
6. At `22:34:20`, revision `6` logged a successful API startup. Public
   `/api/health` and `/api/ready` checks both succeeded.
7. A new signup attempt at `22:36:42` produced the complete SES response:
   `AccessDeniedException`. AWS identified the denied resource as the verified
   destination email identity, not the sender identity or configuration set.
8. The send statement was changed to `Resource: "*"`, retaining the exact
   `ses:FromAddress` and `aws:RequestedRegion` conditions. A saved Terraform
   plan proposed `0 to add, 1 to change, 0 to destroy`.
9. The plan updated only
   `aws_iam_role_policy.api_task` in place. No ECS restart was required.
10. The AWS IAM simulator then returned `allowed` for `ses:SendEmail` when
    evaluated against the previously denied recipient identity with the live
    From-address and Region context. A final Terraform plan reported no changes.
    The correction was committed as `c759c0a`.

## Root cause

The IAM policy modeled `ses:SendEmail` as if restricting the action to the
sender identity were sufficient for every SES request:

```hcl
resources = [
  "arn:aws:ses:eu-west-1:<account>:identity/staging.knitnprint.com",
]
```

For the observed sandbox request, SES performed authorization against the
verified destination identity as well. Because that ARN was outside the policy
statement, AWS rejected the call before accepting the message.

The corrected statement uses a wildcard resource with narrow conditions:

```hcl
statement {
  sid       = "SendStagingTransactionalEmail"
  effect    = "Allow"
  actions   = ["ses:SendEmail"]
  resources = ["*"]

  condition {
    test     = "StringEquals"
    variable = "ses:FromAddress"
    values   = ["no-reply@staging.knitnprint.com"]
  }

  condition {
    test     = "StringEquals"
    variable = "aws:RequestedRegion"
    values   = ["eu-west-1"]
  }
}
```

The wildcard is intentional: recipient identities vary, while the security
boundary that the application must not cross is the permitted sender and
Region. SES sandbox verification remains a separate service-level restriction.

## Contributing factors

### The signup response concealed delivery failure

Account persistence and email delivery are deliberately not one transaction.
The API logged the failed send but still returned success, so the browser could
not distinguish “account created and email accepted” from “account created and
email rejected.” This also made a retry with the same email impossible without
using resend or deleting the staging account.

### Initial logs discarded the useful AWS error

The account-email path formatted the AWS SDK error with its short display form.
That reduced a detailed `403 AccessDeniedException`, AWS request ID, and denied
resource to the generic phrase `service error`.

### The first IAM correction did not test a destination resource

Initial investigation focused on the verified sender identity and SES
configuration-set ARN. Separating those permissions was valid, but it did not
address the destination identity that SES named only after detailed SDK logging
was deployed.

### No end-to-end staging email smoke test existed

Infrastructure validation, API health checks, and IAM simulation did not create
a signup and assert that SES accepted its verification email. Manual testing was
the first check that exercised the complete path with a sandbox recipient.

### Application and SES allowlists were easy to conflate

The staging application allowlist had been removed correctly. SES sandbox
recipient verification is an independent AWS restriction, so removing one did
not make arbitrary staging recipients deliverable.

## Resolution and verification

The following changes are deployed:

- `3f3f335`: split sender/configuration-set authorization and retain detailed
  AWS SDK errors for account-email failures.
- `a2edc9c`: pin the diagnostic API image deployed as ECS revision `6`.
- `c759c0a`: allow recipient resources while constraining the From address and
  AWS Region.

Verification completed:

- Terraform configuration formatting and validation passed.
- Focused backend email tests passed: 8 tests, 0 failures.
- Strict Rust linting passed.
- ECS application revision `6` stabilized with one desired and running task.
- Public API health and database-readiness endpoints returned success.
- The live AWS error identified the destination identity as the denied resource.
- IAM simulation now allows the exact recipient resource that was denied.
- The IAM-only Terraform apply changed one resource in place and destroyed
  nothing.
- A post-apply Terraform plan reported no drift.

One final verification remains: create or resend a verification email to an
SES-verified staging recipient and record successful receipt. Until that check
is recorded, this postmortem remains `remediated; end-to-end inbox confirmation
pending` rather than fully closed.

## Verification commands

### Inspect recent account-email failures

```bash
aws logs tail /aws/ecs/knitnprint-staging/api \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --since 30m \
  --format short
```

Do not copy the full debug response into tickets or chat. It can contain the
recipient address and raw AWS response metadata.

### Simulate the corrected recipient authorization

```bash
aws iam simulate-principal-policy \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --policy-source-arn \
    arn:aws:iam::<account>:role/knitnprint-staging-api-task \
  --action-names ses:SendEmail \
  --resource-arns \
    arn:aws:ses:eu-west-1:<account>:identity/<verified-recipient> \
  --context-entries \
    ContextKeyName=ses:FromAddress,ContextKeyValues=no-reply@staging.knitnprint.com,ContextKeyType=string \
    ContextKeyName=aws:RequestedRegion,ContextKeyValues=eu-west-1,ContextKeyType=string \
  --query 'EvaluationResults[0].EvalDecision' \
  --output text
```

Expected result: `allowed`.

### Confirm infrastructure convergence

```bash
terraform -chdir=infrastructure/terraform/staging/application \
  plan -detailed-exitcode -no-color
```

Expected result: exit code `0` and `No changes`.

## Corrective actions

Completed:

- Corrected SES task-role authorization for verified sandbox recipients.
- Kept the sender and Region restrictions in IAM.
- Kept configuration-set authorization isolated in its own statement.
- Deployed detailed account-email diagnostics.
- Added an admin account-erasure path so staging signup tests can safely reuse an
  email address.
- Removed the staging application email allowlist; SES sandbox verification is
  now the only recipient restriction.

Follow-up:

- Record a successful post-fix inbox delivery and change this document's status
  to `resolved`.
- Add a release smoke test that performs signup or resend with a dedicated,
  SES-verified staging mailbox and asserts SES acceptance.
- Emit structured, redacted SES error fields such as error code and AWS request
  ID instead of retaining the full SDK debug response in normal logs.
- Add a metric and alarm for account-email and notification-worker delivery
  failures.
- Make the signup UI explicitly distinguish account creation from email-delivery
  acceptance and offer resend guidance when delivery fails.
- Add an IAM regression check that evaluates both sender and sandbox recipient
  resources.
- Revisit the equivalent production IAM policy when the production domain is
  configured and the SES production-access request is submitted.
