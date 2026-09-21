output "storefront_certificate_arn" {
  description = "Ireland ACM certificate ARN for the staging storefront and future ALB."
  value       = aws_acm_certificate.storefront.arn
}

output "storefront_acm_dns_records" {
  description = "CNAME records to publish at the external DNS provider for the storefront certificate."
  value = [
    for option in aws_acm_certificate.storefront.domain_validation_options : {
      name  = option.resource_record_name
      type  = option.resource_record_type
      value = option.resource_record_value
    }
  ]
}

output "admin_certificate_arn" {
  description = "us-east-1 ACM certificate ARN for the future admin CloudFront distribution."
  value       = aws_acm_certificate.admin.arn
}

output "admin_acm_dns_records" {
  description = "CNAME records to publish at the external DNS provider for the admin certificate."
  value = [
    for option in aws_acm_certificate.admin.domain_validation_options : {
      name  = option.resource_record_name
      type  = option.resource_record_type
      value = option.resource_record_value
    }
  ]
}

output "ses_identity_arn" {
  description = "Ireland SES identity ARN for staging email."
  value       = aws_sesv2_email_identity.staging.arn
}

output "ses_dkim_dns_records" {
  description = "Easy DKIM CNAME records to publish at the external DNS provider."
  value = [
    for token in aws_sesv2_email_identity.staging.dkim_signing_attributes[0].tokens : {
      name  = "${token}._domainkey.${var.ses_domain}"
      type  = "CNAME"
      value = "${token}.${var.ses_dkim_signing_hosted_zone}"
    }
  ]
}

output "ses_mail_from_dns_records" {
  description = "Custom MAIL FROM MX and SPF records to publish at the external DNS provider."
  value = [
    {
      name     = var.mail_from_domain
      type     = "MX"
      priority = 10
      value    = "feedback-smtp.${var.aws_region}.amazonses.com"
    },
    {
      name     = var.mail_from_domain
      type     = "TXT"
      priority = null
      value    = "v=spf1 include:amazonses.com ~all"
    },
  ]
}

output "ses_configuration_set_name" {
  description = "Value for the application SES_CONFIGURATION_SET environment variable."
  value       = aws_sesv2_configuration_set.transactional.configuration_set_name
}

output "email_failure_topic_arn" {
  description = "SNS topic that receives configured SES failure events."
  value       = aws_sns_topic.email_failures.arn
}
