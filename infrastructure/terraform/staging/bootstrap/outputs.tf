# Staging bootstrap outputs.
output "state_bucket_name" {
  description = "S3 bucket to use for the remote state of subsequent Terraform stacks."
  value       = aws_s3_bucket.terraform_state.id
}

output "budget_name" {
  description = "Account-wide staging budget created by this bootstrap stack."
  value       = aws_budgets_budget.staging_monthly.name
}
