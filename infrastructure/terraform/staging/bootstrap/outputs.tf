# Staging bootstrap outputs.
output "state_bucket_name" {
  description = "S3 bucket to use for the remote state of subsequent Terraform stacks."
  value       = aws_s3_bucket.terraform_state.id
}

output "deployer_permission_set_name" {
  description = "IAM Identity Center permission set to select when configuring the staging CLI profile."
  value       = aws_ssoadmin_permission_set.staging_deployer.name
}

output "deployer_role_arn_pattern" {
  description = "Expected ARN pattern for the IAM role provisioned by the account assignment."
  value       = "arn:${local.partition}:iam::${local.account_id}:role/aws-reserved/sso.amazonaws.com/AWSReservedSSO_${var.deployer_permission_set_name}_*"
}

output "budget_name" {
  description = "Account-wide staging budget created by this bootstrap stack."
  value       = aws_budgets_budget.staging_monthly.name
}
