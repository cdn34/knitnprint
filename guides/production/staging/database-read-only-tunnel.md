# Staging read-only database access through Systems Manager

This guide sets up a personal PostgreSQL read-only login and a tunnel from your
laptop through a dedicated EC2 instance to private staging RDS. These are setup
instructions; the resources below have not been provisioned by adding this guide.

```text
Laptop psql / database client
  → local port 15432 → Systems Manager session → EC2 → private RDS:5432
```

The tunnel provides connectivity. PostgreSQL grants enforce read-only access.
The existing `knitnprint_reporting` NOLOGIN role already has grants in
`infrastructure/terraform/staging/application/database-operations.tf`; a personal
LOGIN role inherits them. It can read all application tables, including customer
and authentication-related data. Use narrower table/view grants instead if that
scope is inappropriate for the person receiving access.

## 1. Install local tools and authenticate

Run commands from the repository root. On macOS with Homebrew:

```bash
brew install awscli jq libpq
brew install --cask session-manager-plugin
export PATH="$(brew --prefix libpq)/bin:$PATH"
aws --version
psql --version
session-manager-plugin --version

export AWS_PROFILE=knitnprint-administrator
export AWS_REGION=eu-west-1
export AWS_PAGER=''
aws sso login --profile "$AWS_PROFILE"
aws sts get-caller-identity
TF_ROOT=infrastructure/terraform/staging/application
terraform -chdir="$TF_ROOT" init
```

Use account `739863594156` and the existing Terraform state. Install the Terraform
version required by `versions.tf` if necessary. The administrator profile is for
one-time setup. Giving someone read-only SQL credentials does not require giving
them this AWS administrator profile; see the permission separation below.

## 2. Add the tunnel host to Terraform

Create `infrastructure/terraform/staging/application/database-access.tf` with the
following contents. This is a proposed addition for you to implement; it is not
already present. It uses existing VPC, subnet, provider, and naming locals.

```hcl
data "aws_ami" "database_access" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["al2023-ami-2023.*-kernel-6.1-x86_64"]
  }
  filter {
    name   = "architecture"
    values = ["x86_64"]
  }
  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

resource "aws_iam_role" "database_access" {
  name = "${local.name_prefix}-database-access"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "ec2.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "database_access_ssm" {
  role       = aws_iam_role.database_access.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
}

resource "aws_iam_instance_profile" "database_access" {
  name = "${local.name_prefix}-database-access"
  role = aws_iam_role.database_access.name
}

resource "aws_security_group" "database_access" {
  name        = "${local.name_prefix}-database-access"
  description = "SSM database tunnel, no inbound connections"
  vpc_id      = aws_vpc.staging.id

  egress {
    description = "Systems Manager and operating system updates"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    description = "Private PostgreSQL"
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }
}

resource "aws_instance" "database_access" {
  ami                         = data.aws_ami.database_access.id
  instance_type               = "t3.micro"
  subnet_id                   = aws_subnet.staging[sort(keys(local.public_subnets))[0]].id
  associate_public_ip_address = true
  vpc_security_group_ids      = [aws_security_group.database_access.id]
  iam_instance_profile        = aws_iam_instance_profile.database_access.name
  user_data_replace_on_change = true

  metadata_options {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 1
  }

  credit_specification {
    cpu_credits = "standard"
  }

  root_block_device {
    encrypted             = true
    volume_type           = "gp3"
    volume_size           = 8
    delete_on_termination = true
  }

  user_data = <<-SCRIPT
    #!/bin/bash
    set -euo pipefail
    dnf upgrade -y
    systemctl enable --now amazon-ssm-agent
  SCRIPT

  tags = { Name = "${local.name_prefix}-database-access" }

  depends_on = [
    aws_iam_role_policy_attachment.database_access_ssm,
    aws_route.public_ipv4,
    aws_route_table_association.public,
  ]
}

output "database_access_instance_id" {
  description = "SSM intermediary for private PostgreSQL access."
  value       = aws_instance.database_access.id
}
```

This public-subnet host has a public IP only for outbound access to SSM and update
endpoints. Its security group has **no inbound rules**, including no SSH rule, and
it has no SSH key. RDS stays in its isolated subnets with `publicly_accessible = false`.
The EC2 role can communicate with SSM; it cannot retrieve database secrets.
Use a current AMI/SSM Agent and maintain OS updates after initial provisioning.
The most-recent AMI lookup can cause replacement in a later Terraform plan; review it.

In the existing `aws_security_group.database` block in `security-groups.tf`, add
this second inline ingress block alongside the existing application ingress:

```hcl
  ingress {
    description     = "PostgreSQL from SSM tunnel host"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.database_access.id]
  }
```

Keep the existing application ingress. Do not mix a separately managed ingress
rule with the inline rules in this security group, or make RDS public.

```bash
terraform -chdir="$TF_ROOT" fmt
terraform -chdir="$TF_ROOT" validate
terraform -chdir="$TF_ROOT" plan -out=database-access.tfplan
terraform -chdir="$TF_ROOT" show -no-color database-access.tfplan
```

Expect the EC2 host, its role/profile/policy attachment/security group, and an RDS
security-group rule. Review other pending changes separately: for example, the
100%/200% ECS healthy-percent change may also appear. Stop if the plan would replace
RDS, change application images unexpectedly, or delete unrelated infrastructure.

```bash
terraform -chdir="$TF_ROOT" apply database-access.tfplan
INSTANCE_ID="$(terraform -chdir="$TF_ROOT" output -raw database_access_instance_id)"
aws ec2 wait instance-status-ok --instance-ids "$INSTANCE_ID"
aws ssm describe-instance-information \
  --filters "Key=InstanceIds,Values=$INSTANCE_ID" \
  --query 'InstanceInformationList[].{id:InstanceId,status:PingStatus,agent:AgentVersion}'
```

Wait for SSM `Online` and initial updates to finish. If the result is empty, check
the instance profile, public subnet route, public IP, HTTPS egress, and SSM Agent.
This host incurs EC2, EBS, and public IPv4 charges. Stop it when unused if desired.

## 3. Open the tunnel in terminal A

```bash
export AWS_PROFILE=knitnprint-administrator
export AWS_REGION=eu-west-1
export AWS_PAGER=''
TF_ROOT=infrastructure/terraform/staging/application
INSTANCE_ID="$(terraform -chdir="$TF_ROOT" output -raw database_access_instance_id)"
DB_HOST="$(terraform -chdir="$TF_ROOT" output -raw database_address)"

aws ssm start-session --target "$INSTANCE_ID" \
  --document-name AWS-StartPortForwardingSessionToRemoteHost \
  --parameters "$(jq -nc --arg host "$DB_HOST" \
    '{host:[$host],portNumber:["5432"],localPortNumber:["15432"]}')"
```

Keep this terminal open. Port `15432` avoids the local development PostgreSQL port
`5432`. Ctrl-C closes the session. Remote-host forwarding needs SSM Agent
3.1.1374.0 or later; use a current patched version rather than that minimum.

## 4. Connect as administrator once, in terminal B

Use the master credential only to set up the PostgreSQL login. Do not share it
with readers. Download the RDS CA bundle and configure libpq to connect through
localhost while verifying the actual RDS hostname:

```bash
export AWS_PROFILE=knitnprint-administrator
export AWS_REGION=eu-west-1
export AWS_PAGER=''
export PATH="$(brew --prefix libpq)/bin:$PATH"
TF_ROOT=infrastructure/terraform/staging/application
DB_HOST="$(terraform -chdir="$TF_ROOT" output -raw database_address)"
mkdir -p "$HOME/.postgresql"
curl --fail --silent --show-error \
  https://truststore.pki.rds.amazonaws.com/eu-west-1/eu-west-1-bundle.pem \
  --output "$HOME/.postgresql/eu-west-1-bundle.pem"

export PGHOST="$DB_HOST"
export PGHOSTADDR=127.0.0.1
export PGPORT=15432
export PGDATABASE=knitnprint
export PGSSLMODE=verify-full
export PGSSLROOTCERT="$HOME/.postgresql/eu-west-1-bundle.pem"

MASTER_SECRET_ARN="$(terraform -chdir="$TF_ROOT" output -raw database_master_secret_arn)"
MASTER_CREDENTIALS="$(aws secretsmanager get-secret-value \
  --secret-id "$MASTER_SECRET_ARN" --query SecretString --output text)"
export PGUSER="$(printf '%s' "$MASTER_CREDENTIALS" | jq -er '.username')"
export PGPASSWORD="$(printf '%s' "$MASTER_CREDENTIALS" | jq -er '.password')"
unset MASTER_CREDENTIALS
PSQL_HISTORY=/dev/null psql -X --set ON_ERROR_STOP=1
```

Do not enable shell tracing (`set -x`). The password stays out of command-line
arguments and files. After exiting this administrator session, run
`unset PGPASSWORD PGUSER MASTER_SECRET_ARN` immediately.

## 5. Create the personal read-only login

Inside the administrator `psql` session, inspect the existing reporting role:

```sql
\du knitnprint_reporting
SELECT pg_get_userbyid(relowner) AS owner, count(*) AS tables
FROM pg_class
WHERE relnamespace = 'public'::regnamespace AND relkind IN ('r', 'p')
GROUP BY relowner;
```

The normal table creator is `knitnprint_migration`. Refresh existing grants and
future-table defaults before creating the personal login:

```sql
BEGIN;
SELECT 'CREATE ROLE knitnprint_reporting NOLOGIN'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'knitnprint_reporting') \gexec
GRANT CONNECT ON DATABASE knitnprint TO knitnprint_reporting;
GRANT USAGE ON SCHEMA public TO knitnprint_reporting;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO knitnprint_reporting;
ALTER DEFAULT PRIVILEGES FOR ROLE knitnprint_migration IN SCHEMA public
  GRANT SELECT ON TABLES TO knitnprint_reporting;
REVOKE CREATE ON SCHEMA public FROM knitnprint_reporting;
REVOKE CREATE, TEMPORARY ON DATABASE knitnprint FROM knitnprint_reporting;

CREATE ROLE carlos_readonly LOGIN INHERIT
  NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
GRANT knitnprint_reporting TO carlos_readonly;
ALTER ROLE carlos_readonly SET default_transaction_read_only = on;
ALTER ROLE carlos_readonly SET statement_timeout = '30s';
ALTER ROLE carlos_readonly SET idle_in_transaction_session_timeout = '60s';
COMMIT;
\password carlos_readonly
```

Choose a new password in the hidden prompts and save it in your password manager.
The `CREATE ROLE` is a one-time command; do not rerun it for an existing login.
If the transaction fails, issue `ROLLBACK;` and diagnose before continuing.
Create a separate named login for each person rather than sharing this one.

The administrator was granted membership in `knitnprint_migration` by our existing
bootstrap, which permits changing its default privileges. If the grant fails,
verify that bootstrap completed; do not grant the reader migration-role membership.
If future tables are created by a different role, that owner's defaults need the
same SELECT grant. Default privileges do not retroactively grant existing tables.

Read-only defaults help avoid mistakes; SELECT-only privileges enforce the
restriction even when a client disables that default. No sequence USAGE, table
write privileges, role administration, or ownership is granted to the reader.
For a new privileged function or SECURITY DEFINER routine, review its EXECUTE
grants too: table grants alone do not constrain what privileged routines can do.

Exit the administrator session and remove its credentials:

```sql
\q
```

```bash
unset PGPASSWORD PGUSER MASTER_SECRET_ARN
psql -X --username carlos_readonly --password
```

This uses the same `PGHOST`, `PGHOSTADDR`, `PGPORT`, and TLS variables from terminal B.

## 6. Verify reads work and writes are denied

Inside the read-only session:

```sql
SELECT current_user, current_database();
SHOW transaction_read_only;
SELECT ssl, version FROM pg_stat_ssl WHERE pid = pg_backend_pid();
SELECT version, description, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 10;
SELECT count(*) FROM orders;
SELECT
  has_table_privilege(current_user, 'public.orders', 'SELECT') AS can_read,
  has_table_privilege(current_user, 'public.orders', 'UPDATE') AS can_update,
  has_database_privilege(current_user, 'knitnprint', 'CREATE') AS can_create_schema,
  has_schema_privilege(current_user, 'public', 'CREATE') AS can_create_table;
```

Expect SSL `true`, reads to succeed, and the last three privileges to be `false`.
Prove that ACLs deny writes even with the read-only default disabled. The following
zero-row statement cannot modify order rows, and the transaction is rolled back:

```sql
BEGIN READ WRITE;
UPDATE orders SET id = id WHERE false;
ROLLBACK;
```

Expect `permission denied for table orders`. Do not use the account if it can write;
inspect `\du carlos_readonly`, memberships, ownership, and table grants first.

## 7. Everyday connections and desktop clients

Each day, authenticate with your authorized AWS profile, start the instance if
stopped, and open the tunnel from step 3. In a second terminal, connect without
retrieving any administrator secret:

```bash
export PATH="$(brew --prefix libpq)/bin:$PATH"
TF_ROOT=infrastructure/terraform/staging/application
export PGHOST="$(terraform -chdir="$TF_ROOT" output -raw database_address)"
export PGHOSTADDR=127.0.0.1
export PGPORT=15432
export PGDATABASE=knitnprint
export PGSSLMODE=verify-full
export PGSSLROOTCERT="$HOME/.postgresql/eu-west-1-bundle.pem"
DB_HOST="$PGHOST"
unset PGPASSWORD
psql -X --username carlos_readonly --password
```

An operator without Terraform-state access can set `PGHOST` to the RDS hostname
provided by the administrator; reading Terraform state is not necessary for SQL
access. Similarly, the administrator can provide the tunnel's EC2 instance ID.

For DBeaver or another PostgreSQL client, the reliable TLS setup is to keep the RDS
hostname as the client host while resolving it to the local tunnel. Print it:

```bash
printf '127.0.0.1 %s # knitnprint-staging-db-tunnel\n' "$DB_HOST"
sudo nano /etc/hosts
```

Add the printed line to `/etc/hosts` on your laptop only. Configure the client:

| Setting | Value |
|---|---|
| Host | The exact RDS hostname printed as `$DB_HOST` |
| Port | `15432` |
| Database | `knitnprint` |
| Username | `carlos_readonly` |
| Password | Your personal password |
| SSL mode | `verify-full` |
| Root certificate | Absolute path to `~/.postgresql/eu-west-1-bundle.pem` |
| Built-in SSH tunnel | Disabled; Systems Manager is already forwarding |

Remove that hosts-file line when no longer needed or if the RDS endpoint changes.
Do not disable certificate hostname verification to work around a localhost mismatch.
With `psql`, `PGHOSTADDR=127.0.0.1` already avoids this hosts-file change.

## 8. Keep AWS access separate from database access

The EC2 instance profile's `AmazonSSMManagedInstanceCore` policy authorizes the
**agent**; it does not authorize a human to open a tunnel. The setup profile above
is an administrator, independently of PostgreSQL's read-only login.

For another operator, configure a dedicated IAM Identity Center permission set
with `ssm:StartSession` limited to this exact EC2 instance ARN and the
`AWS-StartPortForwardingSessionToRemoteHost` SSM document ARN in `eu-west-1`, plus
`ssmmessages:OpenDataChannel` and session resume/terminate access scoped to that
operator's sessions. Follow [AWS's Session Manager policy examples](https://docs.aws.amazon.com/systems-manager/latest/userguide/getting-started-restrict-access-examples.html)
for your identity type; SSO session ARN patterns must match the actual principal.
Do not give readers Secrets Manager master access, Run Command, arbitrary shell
sessions, or administrator permissions merely to enable the tunnel.

The generic remote-host document lets an authorized caller choose its destination;
the host's egress rules constrain destinations to HTTPS and PostgreSQL in the VPC.
For stricter single-endpoint access, use a custom Session document with fixed RDS
host/port and authorize that document instead. CloudTrail records session API
calls; Session Manager port-forwarding sessions do not log the SQL payload.

## 9. Rotate, revoke, and stop when finished

Rotate the personal password from an administrator `psql` connection:

```sql
\password carlos_readonly
```

To revoke login and end existing connections:

```sql
ALTER ROLE carlos_readonly NOLOGIN;
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE usename = 'carlos_readonly' AND pid <> pg_backend_pid();
REVOKE knitnprint_reporting FROM carlos_readonly;
```

Close terminal A's session with Ctrl-C. To reduce idle compute/public IPv4 costs:

```bash
aws ec2 stop-instances --instance-ids "$INSTANCE_ID"
aws ec2 wait instance-stopped --instance-ids "$INSTANCE_ID"
```

EBS storage still incurs charges while stopped. Next time:

```bash
aws ec2 start-instances --instance-ids "$INSTANCE_ID"
aws ec2 wait instance-status-ok --instance-ids "$INSTANCE_ID"
```

Wait for SSM `Online` again. Do not manually terminate this Terraform-managed host;
for permanent removal, remove the documented Terraform additions and the RDS
ingress block, review a fresh plan, and apply it.

## References

- [AWS remote-host port forwarding](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-sessions-start.html#sessions-remote-port-forwarding)
- [Session Manager local plugin](https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html)
- [SSM instance-profile setup](https://docs.aws.amazon.com/systems-manager/latest/userguide/setup-instance-permissions.html)
- [RDS TLS and CA bundles](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/UsingWithRDS.SSL.html)
- [PostgreSQL connection parameters, including host and hostaddr](https://www.postgresql.org/docs/17/libpq-connect.html)
- [PostgreSQL default privileges and object creators](https://www.postgresql.org/docs/17/sql-alterdefaultprivileges.html)
