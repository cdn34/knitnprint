# Providers used only to bootstrap the staging deployment.
provider "aws" {
  profile = var.aws_profile
  region  = var.aws_region

  default_tags {
    tags = local.common_tags
  }
}

provider "aws" {
  alias   = "identity"
  profile = var.aws_profile
  region  = var.identity_region

  default_tags {
    tags = local.common_tags
  }
}
