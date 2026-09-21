resource "aws_acm_certificate" "storefront" {
  domain_name       = var.storefront_domain
  validation_method = "DNS"
  key_algorithm     = "RSA_2048"

  options {
    certificate_transparency_logging_preference = "ENABLED"
    export                                      = "DISABLED"
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_acm_certificate" "admin" {
  provider = aws.edge

  domain_name       = var.admin_domain
  validation_method = "DNS"
  key_algorithm     = "RSA_2048"

  options {
    certificate_transparency_logging_preference = "ENABLED"
    export                                      = "DISABLED"
  }

  lifecycle {
    create_before_destroy = true
  }
}
