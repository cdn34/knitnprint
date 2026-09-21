provider "aws" {
  profile = var.aws_profile
  region  = var.aws_region

  default_tags {
    tags = local.common_tags
  }
}

provider "aws" {
  alias   = "us_east_1"
  profile = var.aws_profile
  region  = "us-east-1"

  default_tags {
    tags = local.common_tags
  }
}
