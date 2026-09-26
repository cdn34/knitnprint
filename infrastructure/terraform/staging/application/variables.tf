variable "aws_profile" {
  description = "Temporary administrator IAM Identity Center profile used for manually reviewed infrastructure operations."
  type        = string
  default     = "knitnprint-administrator"
}

variable "aws_region" {
  description = "Primary AWS Region for staging."
  type        = string
  default     = "eu-west-1"

  validation {
    condition     = var.aws_region == "eu-west-1"
    error_message = "The staging application must remain in eu-west-1."
  }
}

variable "availability_zones" {
  description = "Two Ireland Availability Zones used by public and isolated subnets."
  type        = list(string)
  default     = ["eu-west-1a", "eu-west-1b"]

  validation {
    condition = (
      length(var.availability_zones) == 2 &&
      length(distinct(var.availability_zones)) == 2 &&
      alltrue([for zone in var.availability_zones : startswith(zone, "eu-west-1")])
    )
    error_message = "Provide exactly two distinct eu-west-1 Availability Zones."
  }
}

variable "vpc_cidr" {
  description = "Non-overlapping CIDR reserved for the staging application VPC."
  type        = string
  default     = "10.40.0.0/20"

  validation {
    condition     = can(cidrsubnet(var.vpc_cidr, 4, 15))
    error_message = "vpc_cidr must be a valid CIDR with room for sixteen child subnets."
  }
}

variable "api_image_digest" {
  description = "Immutable OCI index digest for the reviewed staging API image."
  type        = string
  default     = "sha256:3bfdd65ea94b55306745e061cf043173894dfd712d664ff54062f62f3536f224"

  validation {
    condition     = can(regex("^sha256:[0-9a-f]{64}$", var.api_image_digest))
    error_message = "api_image_digest must be a complete sha256 OCI digest."
  }
}

variable "storefront_image_digest" {
  description = "Immutable OCI index digest for the reviewed staging storefront image."
  type        = string
  default     = "sha256:bbff633c84415d43b552458c33f0452c19ddf7ec018d851c249a58ab83850729"

  validation {
    condition     = can(regex("^sha256:[0-9a-f]{64}$", var.storefront_image_digest))
    error_message = "storefront_image_digest must be a complete sha256 OCI digest."
  }
}

variable "application_desired_count" {
  description = "Number of combined staging application tasks. Staging runs one task after database and Stripe prerequisites pass."
  type        = number
  default     = 1

  validation {
    condition     = contains([0, 1], var.application_desired_count)
    error_message = "Lean staging supports either zero or one application task."
  }
}
