locals {
  clamav_image = "clamav/clamav@sha256:83b0541c2e69bc40b7721b340b97478f790e6126fc13e7b224584c8789d8ef51"
}

resource "aws_ecs_task_definition" "application" {
  family                   = local.name_prefix
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "1024"
  memory                   = "5120"
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.api_task.arn

  runtime_platform {
    cpu_architecture        = "X86_64"
    operating_system_family = "LINUX"
  }

  container_definitions = jsonencode([
    {
      name              = "clamav"
      image             = local.clamav_image
      essential         = true
      cpu               = 512
      memoryReservation = 4096
      portMappings = [
        {
          name          = "clamav"
          containerPort = 3310
          hostPort      = 3310
          protocol      = "tcp"
        },
      ]
      healthCheck = {
        command     = ["CMD-SHELL", "clamdcheck.sh"]
        interval    = 30
        timeout     = 5
        retries     = 5
        startPeriod = 300
      }
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["clamav"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "service"
        }
      }
    },
    {
      name              = "api"
      image             = "${aws_ecr_repository.application["api"].repository_url}@${var.api_image_digest}"
      essential         = true
      cpu               = 256
      memoryReservation = 512
      dependsOn = [
        {
          containerName = "clamav"
          condition     = "HEALTHY"
        },
      ]
      portMappings = [
        {
          name          = "api"
          containerPort = 8080
          hostPort      = 8080
          protocol      = "tcp"
          appProtocol   = "http"
        },
      ]
      environment = [
        { name = "APP_ENV", value = "staging" },
        { name = "HOST", value = "0.0.0.0" },
        { name = "PORT", value = "8080" },
        { name = "TRUST_PROXY_HEADERS", value = "true" },
        { name = "TRUSTED_PROXY_HOPS", value = "1" },
        { name = "WEB_ORIGINS", value = "https://staging.knitnprint.com,https://admin.staging.knitnprint.com" },
        { name = "S3_REGION", value = var.aws_region },
        { name = "S3_BUCKET", value = aws_s3_bucket.private["media"].id },
        { name = "MEDIA_SCANNER_ADDRESS", value = "127.0.0.1:3310" },
        { name = "MEDIA_SCAN_TIMEOUT_SECONDS", value = "30" },
        { name = "EMAIL_DELIVERY", value = "ses" },
        { name = "EMAIL_FROM", value = "no-reply@staging.knitnprint.com" },
        { name = "EMAIL_RECIPIENT_ALLOWLIST", value = "danycar.place@gmail.com" },
        { name = "SES_CONFIGURATION_SET", value = "knitnprint-staging-transactional" },
        { name = "STOREFRONT_BASE_URL", value = "https://staging.knitnprint.com" },
        { name = "RUST_LOG", value = "knitnprint_api=info,tower_http=info" },
      ]
      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.application["database_runtime_url"].arn}:url::"
        },
        {
          name      = "STRIPE_SECRET_KEY"
          valueFrom = "${aws_secretsmanager_secret.application["stripe"].arn}:secret_key::"
        },
        {
          name      = "STRIPE_WEBHOOK_SECRET"
          valueFrom = "${aws_secretsmanager_secret.application["stripe"].arn}:webhook_secret::"
        },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["api"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "service"
        }
      }
    },
    {
      name              = "storefront"
      image             = "${aws_ecr_repository.application["storefront"].repository_url}@${var.storefront_image_digest}"
      essential         = true
      cpu               = 256
      memoryReservation = 256
      portMappings = [
        {
          name          = "storefront"
          containerPort = 3000
          hostPort      = 3000
          protocol      = "tcp"
          appProtocol   = "http"
        },
      ]
      environment = [
        { name = "APP_ENV", value = "staging" },
        { name = "NODE_ENV", value = "production" },
        { name = "HOST", value = "0.0.0.0" },
        { name = "PORT", value = "3000" },
        { name = "API_BASE_URL", value = "http://127.0.0.1:8080" },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["storefront"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "service"
        }
      }
    },
  ])
}

resource "aws_ecs_service" "application" {
  name             = local.name_prefix
  cluster          = aws_ecs_cluster.staging.id
  task_definition  = aws_ecs_task_definition.application.arn
  desired_count    = var.application_desired_count
  launch_type      = "FARGATE"
  platform_version = "1.4.0"

  deployment_minimum_healthy_percent = 0
  deployment_maximum_percent         = 100
  health_check_grace_period_seconds  = 600

  enable_ecs_managed_tags = true
  enable_execute_command  = false
  propagate_tags          = "TASK_DEFINITION"
  wait_for_steady_state   = true

  network_configuration {
    assign_public_ip = true
    security_groups  = [aws_security_group.application.id]
    subnets          = [for name in sort(keys(local.public_subnets)) : aws_subnet.staging[name].id]
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = 8080
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.storefront.arn
    container_name   = "storefront"
    container_port   = 3000
  }

  depends_on = [
    aws_lb_listener.https,
    aws_lb_listener_rule.api,
  ]

  tags = {
    Name = local.name_prefix
  }
}
