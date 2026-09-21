# Inputs for the staging bootstrap only.
variable "aws_profile" {
  description = "IAM Identity Center administrator profile used for human-reviewed infrastructure changes."
  type        = string
  default     = "knitnprint-administrator"
}

variable "aws_region" {
  description = "Region for the Terraform state bucket and staging workload."
  type        = string
  default     = "eu-west-1"
}

variable "budget_notification_email" {
  description = "Email address that receives AWS Budget notifications. Kept in an ignored terraform.tfvars file."
  type        = string

  validation {
    condition     = can(regex("^[^[:space:]@]+@[^[:space:]@]+\\.[^[:space:]@]+$", var.budget_notification_email))
    error_message = "budget_notification_email must be a valid email address."
  }
}

variable "monthly_budget_usd" {
  description = "Account-wide monthly cost budget while this account primarily hosts KnitNPrint staging."
  type        = number
  default     = 120

  validation {
    condition     = var.monthly_budget_usd > 0
    error_message = "monthly_budget_usd must be greater than zero."
  }
}
