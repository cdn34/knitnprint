locals {
  postgres_bootstrap_image = "postgres@sha256:7456ef82e5f5bc43d997f4781bbd7c0d6389bff397564649a356e206ba473aee"

  database_bootstrap_script = <<-SCRIPT
    export PGPASSWORD="$MASTER_PASSWORD"

    psql \
      --host "$DATABASE_HOST" \
      --port "$DATABASE_PORT" \
      --dbname "$DATABASE_NAME" \
      --username "$MASTER_USERNAME" \
      --set ON_ERROR_STOP=1 \
      --set master_username="$MASTER_USERNAME" \
      --set migration_password="$MIGRATION_PASSWORD" \
      --set runtime_password="$RUNTIME_PASSWORD" <<'SQL'
    SELECT format(
      'CREATE ROLE %I LOGIN PASSWORD %L',
      'knitnprint_migration',
      :'migration_password'
    )
    WHERE NOT EXISTS (
      SELECT 1 FROM pg_roles WHERE rolname = 'knitnprint_migration'
    ) \gexec

    SELECT format(
      'ALTER ROLE %I WITH LOGIN PASSWORD %L',
      'knitnprint_migration',
      :'migration_password'
    ) \gexec

    SELECT format(
      'CREATE ROLE %I LOGIN PASSWORD %L',
      'knitnprint_runtime',
      :'runtime_password'
    )
    WHERE NOT EXISTS (
      SELECT 1 FROM pg_roles WHERE rolname = 'knitnprint_runtime'
    ) \gexec

    SELECT format(
      'ALTER ROLE %I WITH LOGIN PASSWORD %L',
      'knitnprint_runtime',
      :'runtime_password'
    ) \gexec

    SELECT 'CREATE ROLE knitnprint_reporting NOLOGIN'
    WHERE NOT EXISTS (
      SELECT 1 FROM pg_roles WHERE rolname = 'knitnprint_reporting'
    ) \gexec

    REVOKE CREATE ON SCHEMA public FROM PUBLIC;
    REVOKE ALL ON DATABASE knitnprint FROM PUBLIC;

    GRANT CONNECT, CREATE, TEMPORARY ON DATABASE knitnprint TO knitnprint_migration;
    GRANT USAGE, CREATE ON SCHEMA public TO knitnprint_migration;
    GRANT CONNECT ON DATABASE knitnprint TO knitnprint_runtime, knitnprint_reporting;
    GRANT USAGE ON SCHEMA public TO knitnprint_runtime, knitnprint_reporting;

    SELECT format('GRANT %I TO %I', 'knitnprint_migration', :'master_username') \gexec

    GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO knitnprint_runtime;
    GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO knitnprint_runtime;
    GRANT SELECT ON ALL TABLES IN SCHEMA public TO knitnprint_reporting;

    ALTER DEFAULT PRIVILEGES FOR ROLE knitnprint_migration IN SCHEMA public
      GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO knitnprint_runtime;
    ALTER DEFAULT PRIVILEGES FOR ROLE knitnprint_migration IN SCHEMA public
      GRANT USAGE, SELECT ON SEQUENCES TO knitnprint_runtime;
    ALTER DEFAULT PRIVILEGES FOR ROLE knitnprint_migration IN SCHEMA public
      GRANT SELECT ON TABLES TO knitnprint_reporting;

    REVOKE CREATE ON SCHEMA public FROM knitnprint_runtime, knitnprint_reporting;
    REVOKE CREATE, TEMPORARY ON DATABASE knitnprint FROM knitnprint_runtime, knitnprint_reporting;
    SQL
  SCRIPT
}

resource "aws_ecs_task_definition" "database_bootstrap" {
  family                   = "${local.name_prefix}-database-bootstrap"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.database_bootstrap_execution.arn

  runtime_platform {
    cpu_architecture        = "X86_64"
    operating_system_family = "LINUX"
  }

  container_definitions = jsonencode([
    {
      name      = "database-bootstrap"
      image     = local.postgres_bootstrap_image
      essential = true
      command   = ["sh", "-ec", local.database_bootstrap_script]
      environment = [
        { name = "DATABASE_HOST", value = aws_db_instance.postgres.address },
        { name = "DATABASE_PORT", value = tostring(aws_db_instance.postgres.port) },
        { name = "DATABASE_NAME", value = aws_db_instance.postgres.db_name },
      ]
      secrets = [
        {
          name      = "MASTER_USERNAME"
          valueFrom = "${aws_db_instance.postgres.master_user_secret[0].secret_arn}:username::"
        },
        {
          name      = "MASTER_PASSWORD"
          valueFrom = "${aws_db_instance.postgres.master_user_secret[0].secret_arn}:password::"
        },
        {
          name      = "MIGRATION_PASSWORD"
          valueFrom = "${aws_secretsmanager_secret.application["database_migration_url"].arn}:password::"
        },
        {
          name      = "RUNTIME_PASSWORD"
          valueFrom = "${aws_secretsmanager_secret.application["database_runtime_url"].arn}:password::"
        },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["database-bootstrap"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "bootstrap"
        }
      }
    },
  ])
}

resource "aws_ecs_task_definition" "database_migration" {
  family                   = "${local.name_prefix}-database-migration"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.ecs_execution.arn

  runtime_platform {
    cpu_architecture        = "X86_64"
    operating_system_family = "LINUX"
  }

  container_definitions = jsonencode([
    {
      name      = "database-migration"
      image     = "${aws_ecr_repository.application["api"].repository_url}@${var.api_image_digest}"
      essential = true
      command   = ["/usr/local/bin/migrate"]
      secrets = [
        {
          name      = "MIGRATION_DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.application["database_migration_url"].arn}:url::"
        },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["api"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "migration"
        }
      }
    },
  ])
}

resource "aws_ecs_task_definition" "owner_bootstrap" {
  family                   = "${local.name_prefix}-owner-bootstrap"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.database_bootstrap_execution.arn

  runtime_platform {
    cpu_architecture        = "X86_64"
    operating_system_family = "LINUX"
  }

  container_definitions = jsonencode([
    {
      name      = "owner-bootstrap"
      image     = "${aws_ecr_repository.application["api"].repository_url}@${var.api_image_digest}"
      essential = true
      command   = ["/usr/local/bin/create_owner"]
      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.application["database_runtime_url"].arn}:url::"
        },
        {
          name      = "OWNER_EMAIL"
          valueFrom = "${aws_secretsmanager_secret.owner_bootstrap.arn}:email::"
        },
        {
          name      = "OWNER_PASSWORD"
          valueFrom = "${aws_secretsmanager_secret.owner_bootstrap.arn}:password::"
        },
        {
          name      = "OWNER_NAME"
          valueFrom = "${aws_secretsmanager_secret.owner_bootstrap.arn}:name::"
        },
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs["api"].name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "owner-bootstrap"
        }
      }
    },
  ])
}
