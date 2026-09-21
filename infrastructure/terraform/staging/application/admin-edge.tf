resource "aws_cloudfront_origin_access_control" "admin" {
  name                              = "${local.name_prefix}-admin"
  description                       = "Private S3 access for the KnitNPrint staging admin application."
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

resource "aws_cloudfront_response_headers_policy" "admin" {
  name    = "${local.name_prefix}-admin-noindex"
  comment = "Security and staging noindex headers for the KnitNPrint admin application."

  custom_headers_config {
    items {
      header   = "X-Robots-Tag"
      value    = "noindex, nofollow, noarchive, nosnippet"
      override = true
    }
  }

  security_headers_config {
    content_type_options {
      override = true
    }

    frame_options {
      frame_option = "DENY"
      override     = true
    }

    referrer_policy {
      referrer_policy = "strict-origin-when-cross-origin"
      override        = true
    }

    strict_transport_security {
      access_control_max_age_sec = 31536000
      include_subdomains         = true
      preload                    = false
      override                   = true
    }
  }
}

resource "aws_cloudfront_distribution" "admin" {
  enabled             = true
  is_ipv6_enabled     = true
  comment             = "KnitNPrint staging admin application"
  default_root_object = "index.html"
  aliases             = ["admin.staging.knitnprint.com"]
  http_version        = "http2and3"
  price_class         = "PriceClass_100"
  wait_for_deployment = true

  origin {
    domain_name              = aws_s3_bucket.private["admin_assets"].bucket_regional_domain_name
    origin_id                = "admin-assets"
    origin_access_control_id = aws_cloudfront_origin_access_control.admin.id
  }

  origin {
    domain_name = "staging.knitnprint.com"
    origin_id   = "staging-api"

    custom_origin_config {
      http_port              = 80
      https_port             = 443
      origin_protocol_policy = "https-only"
      origin_ssl_protocols   = ["TLSv1.2"]
    }
  }

  default_cache_behavior {
    target_origin_id           = "admin-assets"
    viewer_protocol_policy     = "redirect-to-https"
    allowed_methods            = ["GET", "HEAD", "OPTIONS"]
    cached_methods             = ["GET", "HEAD", "OPTIONS"]
    cache_policy_id            = data.aws_cloudfront_cache_policy.caching_optimized.id
    response_headers_policy_id = aws_cloudfront_response_headers_policy.admin.id
    compress                   = true
  }

  ordered_cache_behavior {
    path_pattern             = "/api/*"
    target_origin_id         = "staging-api"
    viewer_protocol_policy   = "https-only"
    allowed_methods          = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"]
    cached_methods           = ["GET", "HEAD", "OPTIONS"]
    cache_policy_id          = data.aws_cloudfront_cache_policy.caching_disabled.id
    origin_request_policy_id = data.aws_cloudfront_origin_request_policy.all_viewer_except_host.id
    compress                 = true
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = data.aws_acm_certificate.admin.arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }

  tags = {
    Name = "${local.name_prefix}-admin"
  }
}
