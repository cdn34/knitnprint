data "aws_caller_identity" "current" {}

data "aws_partition" "current" {}

locals {
  account_id       = data.aws_caller_identity.current.account_id
  partition        = data.aws_partition.current.partition
  application_name = "knitnprint"
  environment      = "staging"
  name_prefix      = "${local.application_name}-${local.environment}"

  configuration_set_name = "${local.name_prefix}-transactional"
  email_topic_name       = "${local.name_prefix}-email-failures"

  common_tags = {
    Application = local.application_name
    Environment = local.environment
    ManagedBy   = "terraform"
    Stack       = "staging-dns"
  }
}
