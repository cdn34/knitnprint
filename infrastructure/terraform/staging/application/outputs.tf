output "vpc_id" {
  description = "Staging application VPC ID."
  value       = aws_vpc.staging.id
}

output "public_subnet_ids" {
  description = "Public subnet IDs for the ALB and lean staging Fargate tasks."
  value       = [for name in sort(keys(local.public_subnets)) : aws_subnet.staging[name].id]
}

output "database_subnet_ids" {
  description = "Isolated subnet IDs reserved for RDS."
  value       = [for name in sort(keys(local.database_subnets)) : aws_subnet.staging[name].id]
}

output "security_group_ids" {
  description = "Security-group IDs keyed by workload purpose."
  value = {
    load_balancer = aws_security_group.load_balancer.id
    application   = aws_security_group.application.id
    database      = aws_security_group.database.id
  }
}

output "availability_zones" {
  description = "Availability Zones selected for the staging network."
  value       = var.availability_zones
}

output "media_bucket_name" {
  description = "Private S3 bucket used for product, category, and customer media."
  value       = aws_s3_bucket.private["media"].id
}

output "admin_assets_bucket_name" {
  description = "Private S3 bucket used for the compiled admin application."
  value       = aws_s3_bucket.private["admin_assets"].id
}

output "admin_cloudfront" {
  description = "CloudFront distribution serving the private staging admin SPA and proxying its API requests."
  value = {
    id          = aws_cloudfront_distribution.admin.id
    domain_name = aws_cloudfront_distribution.admin.domain_name
    dns_record = {
      type  = "CNAME"
      host  = "admin.staging"
      value = aws_cloudfront_distribution.admin.domain_name
    }
  }
}

output "ecr_repository_urls" {
  description = "ECR repository URLs keyed by application service."
  value       = { for name, repository in aws_ecr_repository.application : name => repository.repository_url }
}

output "database_address" {
  description = "Private RDS hostname used by application tasks inside the VPC."
  value       = aws_db_instance.postgres.address
}

output "database_port" {
  description = "PostgreSQL port."
  value       = aws_db_instance.postgres.port
}

output "database_master_secret_arn" {
  description = "ARN of the RDS-managed master credential secret; the secret value is not stored in Terraform output."
  value       = aws_db_instance.postgres.master_user_secret[0].secret_arn
}

output "load_balancer" {
  description = "Public staging ALB identifiers used by external DNS and later CloudFront configuration."
  value = {
    dns_name = aws_lb.application.dns_name
    zone_id  = aws_lb.application.zone_id
  }
}

output "target_group_arns" {
  description = "Target groups used by the later ECS services."
  value = {
    api        = aws_lb_target_group.api.arn
    storefront = aws_lb_target_group.storefront.arn
  }
}

output "ecs_cluster" {
  description = "ECS cluster used by the staging services and one-off operational tasks."
  value = {
    arn  = aws_ecs_cluster.staging.arn
    name = aws_ecs_cluster.staging.name
  }
}

output "ecs_role_arns" {
  description = "IAM roles used by the ECS agent and API workload."
  value = {
    api_task                     = aws_iam_role.api_task.arn
    execution                    = aws_iam_role.ecs_execution.arn
    database_bootstrap_execution = aws_iam_role.database_bootstrap_execution.arn
  }
}

output "application_secret_arns" {
  description = "Secret containers that must be populated outside Terraform before tasks are started."
  value       = { for name, secret in aws_secretsmanager_secret.application : name => secret.arn }
}

output "owner_bootstrap_secret_arn" {
  description = "Temporary owner credential secret ARN; its value is populated outside Terraform and removed after bootstrap."
  value       = aws_secretsmanager_secret.owner_bootstrap.arn
}

output "ecs_log_groups" {
  description = "Fourteen-day CloudWatch log groups used by the staging containers."
  value       = { for name, group in aws_cloudwatch_log_group.ecs : name => group.name }
}

output "ecs_task_definitions" {
  description = "Task-definition ARNs for the stopped service and reviewed one-off database operations."
  value = {
    application        = aws_ecs_task_definition.application.arn
    database_bootstrap = aws_ecs_task_definition.database_bootstrap.arn
    database_migration = aws_ecs_task_definition.database_migration.arn
    owner_bootstrap    = aws_ecs_task_definition.owner_bootstrap.arn
  }
}

output "ecs_service" {
  description = "Lean staging service state."
  value = {
    name          = aws_ecs_service.application.name
    desired_count = aws_ecs_service.application.desired_count
  }
}
