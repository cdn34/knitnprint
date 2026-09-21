provider "aws" {
  profile = var.aws_profile
  region  = var.aws_region

  default_tags {
    tags = local.common_tags
  }
}

provider "aws" {
  alias   = "edge"
  profile = var.aws_profile
  region  = var.edge_region

  default_tags {
    tags = local.common_tags
  }
}
