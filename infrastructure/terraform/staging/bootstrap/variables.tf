# Inputs for the staging bootstrap only.
variable "aws_profile" {
  description = "Existing IAM Identity Center administrator profile used only to bootstrap deployment access and state."
  type        = string
  default     = "knitnprint-administrator"
}

variable "aws_region" {
  description = "Region for the Terraform state bucket and staging workload."
  type        = string
  default     = "eu-west-1"
}

variable "identity_region" {
  description = "Region containing the IAM Identity Center organization instance."
  type        = string
  default     = "us-east-1"
}

variable "administrator_group_name" {
  description = "Existing IAM Identity Center group that receives the staging deployer permission set."
  type        = string
  default     = "knitnprint-administrators"
}

variable "deployer_permission_set_name" {
  description = "Name of the IAM Identity Center permission set used for staging Terraform operations."
  type        = string
  default     = "KnitNPrintStagingDeployer"
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
