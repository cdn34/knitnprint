data "aws_iam_policy_document" "ecs_tasks_assume_role" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "ecs_execution" {
  name               = "${local.name_prefix}-ecs-execution"
  description        = "ECS agent access for KnitNPrint staging image pulls, logs, and named secret injection."
  assume_role_policy = data.aws_iam_policy_document.ecs_tasks_assume_role.json
}

resource "aws_iam_role_policy_attachment" "ecs_execution" {
  role       = aws_iam_role.ecs_execution.name
  policy_arn = "arn:${data.aws_partition.current.partition}:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

data "aws_iam_policy_document" "ecs_execution_secrets" {
  statement {
    sid       = "ReadNamedStagingApplicationSecrets"
    effect    = "Allow"
    actions   = ["secretsmanager:GetSecretValue"]
    resources = [for secret in aws_secretsmanager_secret.application : secret.arn]
  }
}

resource "aws_iam_role_policy" "ecs_execution_secrets" {
  name   = "${local.name_prefix}-named-secrets"
  role   = aws_iam_role.ecs_execution.id
  policy = data.aws_iam_policy_document.ecs_execution_secrets.json
}

# This role is used only by one-off bootstrap tasks. Keeping the RDS master and
# initial owner credentials out of the normal service execution role prevents
# a routine application rollout from injecting them into a container definition.
resource "aws_iam_role" "database_bootstrap_execution" {
  name               = "${local.name_prefix}-database-bootstrap-execution"
  description        = "One-off ECS agent access for staging database and owner bootstrap tasks."
  assume_role_policy = data.aws_iam_policy_document.ecs_tasks_assume_role.json
}

resource "aws_iam_role_policy_attachment" "database_bootstrap_execution" {
  role       = aws_iam_role.database_bootstrap_execution.name
  policy_arn = "arn:${data.aws_partition.current.partition}:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

data "aws_iam_policy_document" "database_bootstrap_secrets" {
  statement {
    sid     = "ReadStagingDatabaseBootstrapSecrets"
    effect  = "Allow"
    actions = ["secretsmanager:GetSecretValue"]
    resources = [
      aws_db_instance.postgres.master_user_secret[0].secret_arn,
      aws_secretsmanager_secret.application["database_migration_url"].arn,
      aws_secretsmanager_secret.application["database_runtime_url"].arn,
      aws_secretsmanager_secret.owner_bootstrap.arn,
    ]
  }
}

resource "aws_iam_role_policy" "database_bootstrap_secrets" {
  name   = "${local.name_prefix}-database-bootstrap-secrets"
  role   = aws_iam_role.database_bootstrap_execution.id
  policy = data.aws_iam_policy_document.database_bootstrap_secrets.json
}

resource "aws_iam_role" "api_task" {
  name               = "${local.name_prefix}-api-task"
  description        = "Runtime AWS access for the KnitNPrint staging API task."
  assume_role_policy = data.aws_iam_policy_document.ecs_tasks_assume_role.json
}

data "aws_iam_policy_document" "api_task" {
  statement {
    sid    = "ReadStagingMediaBucketMetadata"
    effect = "Allow"
    actions = [
      "s3:GetBucketLocation",
      "s3:ListBucket",
    ]
    resources = [aws_s3_bucket.private["media"].arn]
  }

  statement {
    sid    = "ManageStagingMediaObjects"
    effect = "Allow"
    actions = [
      "s3:DeleteObject",
      "s3:GetObject",
      "s3:PutObject",
    ]
    resources = ["${aws_s3_bucket.private["media"].arn}/*"]
  }

  statement {
    sid     = "SendStagingTransactionalEmail"
    effect  = "Allow"
    actions = ["ses:SendEmail"]
    resources = [
      "arn:${data.aws_partition.current.partition}:ses:${var.aws_region}:${local.account_id}:configuration-set/knitnprint-staging-transactional",
      "arn:${data.aws_partition.current.partition}:ses:${var.aws_region}:${local.account_id}:identity/staging.knitnprint.com",
    ]

    condition {
      test     = "StringEquals"
      variable = "ses:FromAddress"
      values   = ["no-reply@staging.knitnprint.com"]
    }

    condition {
      test     = "StringEquals"
      variable = "aws:RequestedRegion"
      values   = [var.aws_region]
    }
  }
}

resource "aws_iam_role_policy" "api_task" {
  name   = "${local.name_prefix}-api-runtime"
  role   = aws_iam_role.api_task.id
  policy = data.aws_iam_policy_document.api_task.json
}
