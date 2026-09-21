variable "aws_profile" {
  description = "IAM Identity Center administrator profile used for human-reviewed infrastructure changes."
  type        = string
  default     = "knitnprint-administrator"
}

variable "aws_region" {
  description = "Primary Region for staging and Amazon SES."
  type        = string
  default     = "eu-west-1"
}

variable "edge_region" {
  description = "Region required for the future CloudFront ACM certificate."
  type        = string
  default     = "us-east-1"
}

variable "storefront_domain" {
  description = "Public staging storefront hostname."
  type        = string
  default     = "staging.knitnprint.com"
}

variable "admin_domain" {
  description = "Public staging administration hostname."
  type        = string
  default     = "admin.staging.knitnprint.com"
}

variable "ses_domain" {
  description = "Dedicated SES sending identity for staging."
  type        = string
  default     = "staging.knitnprint.com"
}

variable "mail_from_domain" {
  description = "Custom MAIL FROM subdomain used for staging email alignment."
  type        = string
  default     = "bounce.staging.knitnprint.com"
}

variable "ses_dkim_signing_hosted_zone" {
  description = "SES SigningHostedZone returned for Easy DKIM in eu-west-1; verify this live after identity creation."
  type        = string
  default     = "dkim.amazonses.com"
}
