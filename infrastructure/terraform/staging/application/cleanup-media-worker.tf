resource "aws_scheduler_schedule_group" "media_cleanup" {
  name = "${local.name_prefix}-media-cleanup"
}

# This command only deletes storage objects; it does not read/write files or send email.
data "aws_iam_policy_document" "media_cleanup_task_assume_role" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }

    condition {
      test     = "StringEquals"
      variable = "aws:SourceAccount"
      values   = [local.account_id]
    }

    condition {
      test     = "ArnLike"
      variable = "aws:SourceArn"
      values   = ["arn:${data.aws_partition.current.partition}:ecs:${var.aws_region}:${local.account_id}:*"]
    }
  }
}

resource "aws_iam_role" "media_cleanup_task" {
  name               = "${local.name_prefix}-media-cleanup-task"
  description        = "Delete objects from the staging media bucket for marked and abandoned media cleanup."
  assume_role_policy = data.aws_iam_policy_document.media_cleanup_task_assume_role.json
}

data "aws_iam_policy_document" "media_cleanup_task" {
  statement {
    sid       = "DeleteStagingMediaObjects"
    effect    = "Allow"
    actions   = ["s3:DeleteObject"]
    resources = ["${aws_s3_bucket.private["media"].arn}/*"]
  }
}

resource "aws_iam_role_policy" "media_cleanup_task" {
  name   = "${local.name_prefix}-delete-media-objects"
  role   = aws_iam_role.media_cleanup_task.id
  policy = data.aws_iam_policy_document.media_cleanup_task.json
}

# One-shot worker using the same immutable API image as the application.
resource "aws_ecs_task_definition" "cleanup_media_worker" {
  family                   = "${local.name_prefix}-cleanup-media"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.media_cleanup_task.arn

  runtime_platform {
    cpu_architecture        = "X86_64"
    operating_system_family = "LINUX"
  }

  container_definitions = jsonencode([
    {
      name      = "cleanup-media"
      image     = "${aws_ecr_repository.application["api"].repository_url}@${var.api_image_digest}"
      essential = true
      command   = ["/usr/local/bin/cleanup_media"]
      environment = [
        { name = "APP_ENV", value = "staging" },
        { name = "S3_REGION", value = var.aws_region },
        { name = "S3_BUCKET", value = aws_s3_bucket.private["media"].id },
        { name = "MEDIA_PENDING_MAX_HOURS", value = "24" },
        { name = "RUST_LOG", value = "knitnprint_api=info" },
      ]
      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.application["database_runtime_url"].arn}:url::"
        },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["cleanup-media"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "scheduled"
        }
      }
    },
  ])
}

data "aws_iam_policy_document" "media_cleanup_scheduler_assume_role" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["scheduler.amazonaws.com"]
    }

    condition {
      test     = "StringEquals"
      variable = "aws:SourceAccount"
      values   = [local.account_id]
    }

    # Scheduler supplies the schedule GROUP ARN, not the individual schedule ARN.
    condition {
      test     = "ArnEquals"
      variable = "aws:SourceArn"
      values   = [aws_scheduler_schedule_group.media_cleanup.arn]
    }
  }
}

resource "aws_iam_role" "cleanup_media_scheduler" {
  name               = "${local.name_prefix}-cleanup-media-scheduler"
  description        = "Allows EventBridge Scheduler to launch the staging media cleanup worker."
  assume_role_policy = data.aws_iam_policy_document.media_cleanup_scheduler_assume_role.json
}

data "aws_iam_policy_document" "cleanup_media_scheduler" {
  statement {
    sid       = "RunMediaCleanupWorker"
    effect    = "Allow"
    actions   = ["ecs:RunTask"]
    resources = [aws_ecs_task_definition.cleanup_media_worker.arn]
  }

  statement {
    sid     = "PassMediaCleanupWorkerRoles"
    effect  = "Allow"
    actions = ["iam:PassRole"]
    resources = [
      aws_iam_role.ecs_execution.arn,
      aws_iam_role.media_cleanup_task.arn,
    ]

    condition {
      test     = "StringEquals"
      variable = "iam:PassedToService"
      values   = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role_policy" "cleanup_media_scheduler" {
  name   = "${local.name_prefix}-run-cleanup-media"
  role   = aws_iam_role.cleanup_media_scheduler.id
  policy = data.aws_iam_policy_document.cleanup_media_scheduler.json
}

resource "aws_scheduler_schedule" "cleanup_media_worker" {
  name                         = "${local.name_prefix}-cleanup-media"
  description                  = "Delete marked media assets and abandoned uploads every hour."
  group_name                   = aws_scheduler_schedule_group.media_cleanup.name
  schedule_expression          = "rate(1 hour)"
  schedule_expression_timezone = "UTC"
  state                        = "ENABLED"

  flexible_time_window {
    mode = "OFF"
  }

  target {
    arn      = aws_ecs_cluster.staging.arn
    role_arn = aws_iam_role.cleanup_media_scheduler.arn

    ecs_parameters {
      task_definition_arn = aws_ecs_task_definition.cleanup_media_worker.arn
      launch_type         = "FARGATE"
      platform_version    = "1.4.0"
      task_count          = 1

      network_configuration {
        assign_public_ip = true
        security_groups  = [aws_security_group.application.id]
        subnets          = [for name in sort(keys(local.public_subnets)) : aws_subnet.staging[name].id]
      }
    }

    retry_policy {
      maximum_event_age_in_seconds = 900
      maximum_retry_attempts       = 2
    }
  }

  depends_on = [
    aws_iam_role_policy.cleanup_media_scheduler,
    aws_iam_role_policy.media_cleanup_task,
    aws_iam_role_policy.ecs_execution_secrets,
    aws_iam_role_policy_attachment.ecs_execution,
  ]
}
