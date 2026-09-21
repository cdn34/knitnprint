data "aws_caller_identity" "current" {}

data "aws_partition" "current" {}

data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_acm_certificate" "storefront" {
  domain      = "staging.knitnprint.com"
  statuses    = ["ISSUED"]
  types       = ["AMAZON_ISSUED"]
  most_recent = true
}

data "aws_acm_certificate" "admin" {
  provider = aws.us_east_1

  domain      = "admin.staging.knitnprint.com"
  statuses    = ["ISSUED"]
  types       = ["AMAZON_ISSUED"]
  most_recent = true
}

data "aws_cloudfront_cache_policy" "caching_optimized" {
  name = "Managed-CachingOptimized"
}

data "aws_cloudfront_cache_policy" "caching_disabled" {
  name = "Managed-CachingDisabled"
}

data "aws_cloudfront_origin_request_policy" "all_viewer_except_host" {
  name = "Managed-AllViewerExceptHostHeader"
}

locals {
  application_name = "knitnprint"
  environment      = "staging"
  name_prefix      = "${local.application_name}-${local.environment}"
  account_id       = data.aws_caller_identity.current.account_id

  common_tags = {
    Application = local.application_name
    Environment = local.environment
    ManagedBy   = "terraform"
    Stack       = "staging-application"
  }

  subnets = {
    public-a = {
      availability_zone = var.availability_zones[0]
      netnum            = 0
      public            = true
    }
    public-b = {
      availability_zone = var.availability_zones[1]
      netnum            = 1
      public            = true
    }
    database-a = {
      availability_zone = var.availability_zones[0]
      netnum            = 8
      public            = false
    }
    database-b = {
      availability_zone = var.availability_zones[1]
      netnum            = 9
      public            = false
    }
  }

  public_subnets   = { for name, subnet in local.subnets : name => subnet if subnet.public }
  database_subnets = { for name, subnet in local.subnets : name => subnet if !subnet.public }
  vpc_dns_resolver = "${cidrhost(var.vpc_cidr, 2)}/32"

  private_buckets = {
    media        = "${local.name_prefix}-media-${local.account_id}"
    admin_assets = "${local.name_prefix}-admin-assets-${local.account_id}"
  }

  container_repositories = toset([
    "api",
    "storefront",
  ])
}

check "selected_availability_zones_are_available" {
  assert {
    condition = alltrue([
      for zone in var.availability_zones : contains(data.aws_availability_zones.available.names, zone)
    ])
    error_message = "Every selected Availability Zone must currently be available in eu-west-1."
  }
}
