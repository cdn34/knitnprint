resource "aws_ecs_task_definition" "notification_worker" {
  family                   = "${local.name_prefix}-notification-worker"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.api_task.arn

  runtime_platform {
    cpu_architecture        = "X86_64"
    operating_system_family = "LINUX"
  }

  container_definitions = jsonencode([
    {
      name      = "notification-worker"
      image     = "${aws_ecr_repository.application["api"].repository_url}@${var.api_image_digest}"
      essential = true
      command   = ["/usr/local/bin/deliver_notifications"]
      environment = [
        { name = "APP_ENV", value = "staging" },
        { name = "EMAIL_DELIVERY", value = "ses" },
        { name = "EMAIL_FROM", value = local.staging_email_from },
        { name = "EMAIL_RECIPIENT_ALLOWLIST", value = local.staging_email_allowlist },
        { name = "SES_CONFIGURATION_SET", value = local.staging_ses_configuration_set },
        { name = "STOREFRONT_BASE_URL", value = local.staging_storefront_base_url },
        { name = "NOTIFICATION_BATCH_SIZE", value = "25" },
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
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["notification-worker"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "scheduled"
        }
      }
    },
  ])
}

data "aws_iam_policy_document" "scheduler_assume_role" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["scheduler.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "notification_scheduler" {
  name               = "${local.name_prefix}-notification-scheduler"
  description        = "Allows EventBridge Scheduler to launch the staging notification worker."
  assume_role_policy = data.aws_iam_policy_document.scheduler_assume_role.json
}

data "aws_iam_policy_document" "notification_scheduler" {
  statement {
    sid       = "RunNotificationWorker"
    effect    = "Allow"
    actions   = ["ecs:RunTask"]
    resources = [aws_ecs_task_definition.notification_worker.arn]
  }

  statement {
    sid     = "PassNotificationWorkerRoles"
    effect  = "Allow"
    actions = ["iam:PassRole"]
    resources = [
      aws_iam_role.ecs_execution.arn,
      aws_iam_role.api_task.arn,
    ]

    condition {
      test     = "StringEquals"
      variable = "iam:PassedToService"
      values   = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role_policy" "notification_scheduler" {
  name   = "${local.name_prefix}-run-notification-worker"
  role   = aws_iam_role.notification_scheduler.id
  policy = data.aws_iam_policy_document.notification_scheduler.json
}

resource "aws_scheduler_schedule" "notification_worker" {
  name                         = "${local.name_prefix}-notification-worker"
  description                  = "Drain the KnitNPrint staging transactional-email outbox every five minutes."
  schedule_expression          = "rate(5 minutes)"
  schedule_expression_timezone = "UTC"
  state                        = "ENABLED"

  flexible_time_window {
    mode = "OFF"
  }

  target {
    arn      = aws_ecs_cluster.staging.arn
    role_arn = aws_iam_role.notification_scheduler.arn

    ecs_parameters {
      task_definition_arn     = aws_ecs_task_definition.notification_worker.arn
      launch_type             = "FARGATE"
      platform_version        = "1.4.0"
      task_count              = 1
      enable_ecs_managed_tags = true
      propagate_tags          = "TASK_DEFINITION"

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

  depends_on = [aws_iam_role_policy.notification_scheduler]
}
