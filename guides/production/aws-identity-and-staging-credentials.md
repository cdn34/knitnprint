# AWS identities and staging credentials

Last updated: 2026-09-07

This guide explains the difference between AWS account root, IAM users, IAM Identity Center users, and IAM roles. It also records which identity and credentials the KnitNPrint staging deployment should use.

Official references:

- [Compare IAM identities and credentials](https://docs.aws.amazon.com/IAM/latest/UserGuide/introduction_identity-management.html)
- [IAM Identity Center user credentials](https://docs.aws.amazon.com/singlesignon/latest/userguide/howtogetcredentials.html)
- [IAM roles created by Identity Center](https://docs.aws.amazon.com/singlesignon/latest/userguide/identity-center-and-iam-roles.html)
- [Create a permission set](https://docs.aws.amazon.com/singlesignon/latest/userguide/howtocreatepermissionset.html)
- [Configure user access with the Identity Center directory](https://docs.aws.amazon.com/singlesignon/latest/userguide/quick-start-default-idc.html)
- [AWS CLI SSO configuration](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html)
- [Amazon ECS task IAM roles](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-iam-roles.html)
- [Root-user best practices](https://docs.aws.amazon.com/IAM/latest/UserGuide/root-user-best-practices.html)

## The core difference

IAM users and IAM Identity Center users can both open the AWS Console and run AWS commands. With equivalent permissions, they can manage the same AWS resources. The important difference is how AWS authenticates them, assigns permissions, and issues credentials.

An IAM user is a permanent identity inside one AWS account:

```text
IAM user
  |-- permanent console password
  |-- may have permanent access keys
  `-- policies attached directly or through IAM groups
```

An Identity Center user receives temporary access by assuming a role:

```text
Identity Center user
  |-- signs into the AWS access portal
  |-- selects an account and permission set
  |-- AWS assumes an IAM role
  `-- receives temporary credentials
```

AWS describes this as the main distinction: IAM users can have long-term credentials, while Identity Center users automatically assume IAM roles and receive temporary credentials.

## Identity comparison

| Capability | IAM user | Identity Center user |
| --- | --- | --- |
| Exists in | One AWS account | IAM Identity Center directory |
| Console login | IAM account login page | AWS access portal |
| Password | Permanent IAM-user password | Identity Center or external identity-provider password |
| CLI credentials | Permanent access keys, `aws login`, or assumed roles | Temporary role credentials through `aws sso login` |
| Permanent access keys | Can have them | Cannot have IAM-user access keys |
| Permissions | Policies attached directly or through IAM groups | Permission sets that create IAM roles |
| Multiple AWS accounts | Needs separate users or cross-account roles | One identity can select assigned accounts and roles |
| Credential expiry | Access keys do not expire automatically | Role sessions expire automatically |
| Removal | Delete or disable the user, keys, and password | Disable the user or remove its account/permission-set assignment |
| IAM console visibility | Appears under IAM → Users | Appears under IAM Identity Center → Users |
| Resulting CLI ARN | Usually `...:user/name` or an assumed role | `...:assumed-role/AWSReservedSSO_.../name` |
| Best use | Legacy systems requiring fixed credentials | Human administrators and developers |

Identity Center creates IAM roles beginning with `AWSReservedSSO_` when a permission set is assigned to an AWS account. Identity Center owns those roles and updates their policies when the permission set changes.

## Other identities involved

### AWS account root

The root identity is created with the AWS account and signs in using the account email address. Its permissions cannot be restricted in the normal way. Use it only for root-only tasks and identity bootstrap or recovery when no non-root administrator is available.

The root identity must have MFA and must not have access keys.

### IAM roles

An IAM role has permissions but no permanent password or access key. A trusted person, AWS service, workload, or external identity temporarily assumes it.

Examples for KnitNPrint include:

```text
knitnprint-staging-api-task
knitnprint-staging-task-execution
knitnprint-staging-migration-task
```

### Local AWS CLI profiles

A CLI profile is only a name in the local AWS configuration. It is not an AWS identity and grants no permissions by itself.

Examples:

```text
knitnprint-administrator
```

The profile points the AWS CLI and SDKs toward an authentication method, account, role, and default Region.

## Practical examples

### Example 1: Deploy staging from a development workstation

Both user types could deploy staging if they had equivalent permissions.

An IAM user can be configured with permanent access keys:

```bash
aws configure --profile knitnprint
```

That prompts for:

```text
AWS Access Key ID
AWS Secret Access Key
```

Those values remain valid until they are rotated, disabled, or deleted. They are normally stored in plaintext in the local AWS credentials file. This is not the approach selected for KnitNPrint.

The selected Identity Center profile uses:

```bash
aws sso login --profile knitnprint-administrator
```

A browser opens, the user authenticates, and AWS supplies temporary credentials. The CLI can refresh them without storing a permanent access-key secret.

This is the selected approach for local Terraform work.

### Example 2: A legacy tool requires a fixed access key

An IAM user can be issued a permanent credential pair:

```text
access-key-id
secret-access-key
```

An Identity Center user cannot have a permanent IAM-user access key. It receives temporary role credentials containing:

```text
access-key-id
secret-access-key
session-token
expiration
```

A legacy third-party integration that only understands fixed access keys may require an IAM user. The staging deployment does not currently have such a requirement.

### Example 3: One person manages staging and production

Using environment-named IAM users encourages duplicate permanent identities:

```text
knitnprint-staging
knitnprint-production
```

Identity Center separates the human identity from its temporary permissions:

```text
knitnprint-admin
  `-- AdministratorAccess
```

For this one-person project, the same temporary administrator session can run
reviewed staging and production infrastructure plans. The environments still
have separate state and resources. Runtime services and automated releases use
their own restricted roles.

### Example 4: ECS accesses S3 and SES

Neither an IAM user nor an Identity Center user should provide credentials to an ECS task.

The task receives an IAM task role:

```text
knitnprint-staging-api-task
  |-- read and write the staging media bucket
  `-- send through the staging SES identity
```

ECS supplies and rotates temporary credentials inside the container. Do not place Identity Center credentials, IAM access keys, or AWS passwords in the image, task environment, or Secrets Manager.

Secrets Manager is for application secrets such as database and Stripe credentials, not for AWS task-role credentials.

### Example 5: GitHub Actions deploys later

Neither user type should have credentials copied into GitHub secrets.

The intended flow is:

```text
GitHub workflow
  -> GitHub OIDC identity
  -> temporary AWS deployment role
```

No permanent AWS access key is required.

### Example 6: Perform a root-only account operation

Neither an IAM user nor an Identity Center user replaces root for the small set of operations that require the account root identity. Ordinary AWS auditing, Terraform deployment, ECS operation, S3 access, SES configuration, and RDS management are not reasons to use root.

## Why temporary credentials are safer

Suppose a permanent IAM access key is accidentally:

- committed to Git;
- included in a screenshot;
- printed in CI logs;
- copied into an `.env` file;
- stolen from the AWS credentials file.

It remains usable until someone finds and revokes it.

Identity Center role credentials expire automatically. Stealing them is still serious, but their useful lifetime is bounded. The user's permission-set assignment can also be removed centrally.

## Identity Center Region versus workload Region

The existing IAM Identity Center instance is configured in `us-east-1`. This is its identity-management Region. It stores users, groups, permission sets, and assignments there.

That does not move KnitNPrint resources to Northern Virginia. The staging workload Region remains `eu-west-1`.

Use these separate values during AWS CLI SSO setup:

```text
SSO region:                 us-east-1
CLI default client region:  eu-west-1
Default output format:      json
```

The first tells the CLI where to authenticate. The second tells ordinary AWS service commands where to operate unless a command supplies another `--region`.

## Selected KnitNPrint identity design

Use this Identity Center user for the store administrator:

```text
knitnprint-admin
```

Although its name is store-based, it is an individual login. Do not share its password or MFA. If another administrator joins later, create a separate Identity Center user.

Place the user in this Identity Center group:

```text
Admin
```

Assign that group the predefined permission set named `AdministratorAccess`.
This permission set uses the AWS-managed `AdministratorAccess` policy and is
intentionally broad. The sole human operator uses its temporary SSO sessions
for saved-plan-reviewed infrastructure work. Runtime services and future
deployment automation do not use this permission set.

## Create and assign initial administrator access

Perform this setup in the IAM Identity Center Region, `us-east-1`. These steps require an organization instance with **Multi-account permissions** enabled.

### Create the permission set

1. Open **IAM Identity Center**.
2. Under **Multi-account permissions**, choose **Permission sets**.
3. Choose **Create permission set**.
4. Select **Predefined permission set**.
5. Select the common permission policy **AdministratorAccess**.
6. Keep the initial settings:
   - Permission set name: `AdministratorAccess`
   - Session duration: `1 hour`
   - Relay state: blank
7. Review the configuration and choose **Create**.

Creating a permission set defines the permissions but does not grant them to anybody. It must also be assigned to a user or group for a specific AWS account.

### Assign the permission set to the administrator group

1. Under **Multi-account permissions**, choose **AWS accounts**.
2. Select the checkbox beside the management account.
3. Choose **Assign users or groups**.
4. Open the **Groups** tab.
5. Select `Admin`, then choose **Next**.
6. Select the `AdministratorAccess` permission set, then choose **Next**.
7. Confirm the selected account, group, and permission set, then choose **Submit**.

Also confirm under **Groups → `Admin`** that `knitnprint-admin` is a member. AWS provisions an Identity Center-managed IAM role with an `AWSReservedSSO_` prefix when the account assignment is completed.

The completed bootstrap relationship is:

```text
Identity Center user: knitnprint-admin
  |
  `-- group: Admin
        |
        `-- AWS account assignment
              |
              `-- permission set: AdministratorAccess
```

The selected arrangement is:

```text
knitnprint-admin
  `-- Admin
        `-- AdministratorAccess
              human-reviewed Terraform operations
```

An initially created `KnitNPrintStagingDeployer` permission set was removed on
2026-09-07 because maintaining a large action-by-action policy for a sole human
operator added more complexity than value. This does not change the requirement
for least-privilege ECS task roles or the future GitHub OIDC release role.

## Initial administrator CLI profile

Configure the Identity Center profile:

```bash
aws configure sso --profile knitnprint-administrator
```

Do not run this command with `sudo`; the profile belongs in the current user's AWS CLI configuration. During the wizard use:

```text
SSO session name (Recommended):                         knitnprint
SSO start URL [None]:                                    <AWS access portal URL>
SSO region [None]:                                       us-east-1
SSO registration scopes [sso:account:access]:            sso:account:access
Default client Region [None]:                            eu-west-1
CLI default output format (json if not specified):       json
Profile name [suggested-name]:                           knitnprint-administrator
```

Obtain the start URL from **IAM Identity Center → Settings → AWS access portal URL** instead of guessing it. It normally resembles:

```text
https://d-xxxxxxxxxx.awsapps.com/start
```

Do not use the ordinary AWS Console URL. The wizard opens a browser for authorization; authenticate as the Identity Center user `knitnprint-admin`, not as the AWS account root user.

Always enter `knitnprint` at the first prompt. Leaving the SSO session name blank produces this warning and switches to the legacy, non-refreshable profile format:

```text
WARNING: Configuring using legacy format (e.g. without an SSO session).
Consider re-running "configure sso" command and providing a session name.
```

If that happens, press `Ctrl+C`, rerun the configuration command, and enter `knitnprint` for the session name.

Because the administrator currently has one assigned AWS account and one permission set, the wizard automatically selects the account and reports:

```text
The only role available to you is: AdministratorAccess
Using the role name "AdministratorAccess"
```

If a future user has access to multiple accounts or permission sets, select the intended account and role explicitly.

Sign in:

```bash
aws sso login --profile knitnprint-administrator
```

Verify the resulting temporary identity:

```bash
aws sts get-caller-identity \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager

aws configure get region \
  --profile knitnprint-administrator

aws iam get-account-summary \
  --profile knitnprint-administrator \
  --output json \
  --no-cli-pager
```

The caller ARN should be an assumed `AWSReservedSSO_...` role, not `:root` and not an IAM user ARN.

## Terraform credentials

Use the existing administrator profile for all manually reviewed staging and
production Terraform work:

```bash
aws sso login --profile knitnprint-administrator
```

Verify the caller before Terraform:

```bash
aws sts get-caller-identity \
  --profile knitnprint-administrator \
  --region eu-west-1 \
  --no-cli-pager
```

Run Terraform with the selected profile and Region:

```bash
AWS_PROFILE=knitnprint-administrator \
AWS_REGION=eu-west-1 \
terraform plan
```

The repository's bootstrap, DNS, and application backends already select this
profile. Continue saving and reviewing every plan before apply.

Never put these values into Terraform files, `.tfvars`, `.env` files, Docker images, or GitHub secrets:

```text
AWS_ACCESS_KEY_ID
AWS_SECRET_ACCESS_KEY
AWS_SESSION_TOKEN
```

Identity Center and the AWS SDK credential chain supply temporary values automatically.

## Credential decision summary

| Consumer | Authentication method |
| --- | --- |
| Store administrator | Identity Center user `knitnprint-admin` |
| Bootstrap, audit, and human Terraform | Temporary `AdministratorAccess` session through profile `knitnprint-administrator` |
| ECS API and workers | Per-service ECS task roles |
| ECS image pulls and CloudWatch logs | ECS task execution role |
| GitHub Actions deployment | GitHub OIDC role, added later |
| Application database and Stripe secrets | AWS Secrets Manager |
| AWS root identity | Root-only account, recovery, and emergency operations |
| IAM users with permanent access keys | None unless a future integration proves roles are unsupported |
