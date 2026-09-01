# Account and Identity Center data for the staging bootstrap.
data "aws_caller_identity" "current" {}

data "aws_partition" "current" {}

data "aws_ssoadmin_instances" "current" {
  provider = aws.identity
}

data "aws_identitystore_group" "administrators" {
  provider          = aws.identity
  identity_store_id = tolist(data.aws_ssoadmin_instances.current.identity_store_ids)[0]

  alternate_identifier {
    unique_attribute {
      attribute_path  = "DisplayName"
      attribute_value = var.administrator_group_name
    }
  }
}

locals {
  account_id            = data.aws_caller_identity.current.account_id
  partition             = data.aws_partition.current.partition
  identity_instance_arn = tolist(data.aws_ssoadmin_instances.current.arns)[0]
  application_name      = "knitnprint"
  environment           = "staging"
  name_prefix           = "${local.application_name}-${local.environment}"
  state_bucket_name     = "${local.name_prefix}-terraform-state-${local.account_id}"

  common_tags = {
    Application = local.application_name
    Environment = local.environment
    ManagedBy   = "terraform"
    Stack       = "staging-bootstrap"
  }
}
