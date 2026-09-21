# Account data for the staging bootstrap.
data "aws_caller_identity" "current" {}

data "aws_partition" "current" {}

locals {
  account_id        = data.aws_caller_identity.current.account_id
  partition         = data.aws_partition.current.partition
  application_name  = "knitnprint"
  environment       = "staging"
  name_prefix       = "${local.application_name}-${local.environment}"
  state_bucket_name = "${local.name_prefix}-terraform-state-${local.account_id}"

  common_tags = {
    Application = local.application_name
    Environment = local.environment
    ManagedBy   = "terraform"
    Stack       = "staging-bootstrap"
  }
}
