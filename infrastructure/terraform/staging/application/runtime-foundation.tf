locals {
  ecs_log_groups = toset([
    "api",
    "clamav",
    "database-bootstrap",
    "notification-worker",
    "storefront",
  ])

  application_secrets = {
    database_runtime_url = {
      name        = "knitnprint/staging/database/runtime-url"
      description = "Runtime PostgreSQL URL for KnitNPrint staging."
    }
    database_migration_url = {
      name        = "knitnprint/staging/database/migration-url"
      description = "Migration PostgreSQL URL for KnitNPrint staging."
    }
    stripe = {
      name        = "knitnprint/staging/stripe"
      description = "Stripe test-mode API and webhook credentials for KnitNPrint staging."
    }
  }
}

resource "aws_ecs_cluster" "staging" {
  name = local.name_prefix

  setting {
    name  = "containerInsights"
    value = "disabled"
  }

  tags = {
    Name = local.name_prefix
  }
}

resource "aws_cloudwatch_log_group" "ecs" {
  for_each = local.ecs_log_groups

  name              = "/aws/ecs/${local.name_prefix}/${each.key}"
  retention_in_days = 14
}

resource "aws_secretsmanager_secret" "application" {
  for_each = local.application_secrets

  name                    = each.value.name
  description             = each.value.description
  recovery_window_in_days = 7
}

# The owner password is populated interactively outside Terraform and removed
# after the one-time owner task succeeds. Never store a secret version in state.
resource "aws_secretsmanager_secret" "owner_bootstrap" {
  name                    = "knitnprint/staging/bootstrap/owner"
  description             = "Temporary credentials for the first KnitNPrint staging owner."
  recovery_window_in_days = 7
}
