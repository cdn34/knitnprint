resource "aws_sesv2_configuration_set" "transactional" {
  configuration_set_name = local.configuration_set_name

  delivery_options {
    tls_policy = "REQUIRE"
  }

  reputation_options {
    # SNS captures the actionable failure events. Keep per-configuration-set
    # CloudWatch reputation metrics off until the observability stack defines
    # and budgets for them.
    reputation_metrics_enabled = false
  }

  sending_options {
    sending_enabled = true
  }

  suppression_options {
    suppressed_reasons = ["BOUNCE", "COMPLAINT"]
  }
}

resource "aws_sesv2_account_suppression_attributes" "staging_region" {
  suppressed_reasons = ["BOUNCE", "COMPLAINT"]
}

resource "aws_sesv2_email_identity" "staging" {
  email_identity         = var.ses_domain
  configuration_set_name = aws_sesv2_configuration_set.transactional.configuration_set_name

  dkim_signing_attributes {
    next_signing_key_length = "RSA_2048_BIT"
  }
}

resource "aws_sesv2_email_identity_mail_from_attributes" "staging" {
  email_identity         = aws_sesv2_email_identity.staging.email_identity
  mail_from_domain       = var.mail_from_domain
  behavior_on_mx_failure = "REJECT_MESSAGE"
}

resource "aws_sns_topic" "email_failures" {
  name = local.email_topic_name
}

data "aws_iam_policy_document" "email_failures" {
  statement {
    sid     = "AllowSesConfigurationSetPublishing"
    effect  = "Allow"
    actions = ["sns:Publish"]

    resources = [aws_sns_topic.email_failures.arn]

    principals {
      type        = "Service"
      identifiers = ["ses.amazonaws.com"]
    }

    condition {
      test     = "StringEquals"
      variable = "AWS:SourceAccount"
      values   = [local.account_id]
    }

    condition {
      test     = "StringEquals"
      variable = "AWS:SourceArn"
      values = [
        "arn:${local.partition}:ses:${var.aws_region}:${local.account_id}:configuration-set/${aws_sesv2_configuration_set.transactional.configuration_set_name}",
      ]
    }
  }
}

resource "aws_sns_topic_policy" "email_failures" {
  arn    = aws_sns_topic.email_failures.arn
  policy = data.aws_iam_policy_document.email_failures.json
}

resource "aws_sesv2_configuration_set_event_destination" "email_failures" {
  configuration_set_name = aws_sesv2_configuration_set.transactional.configuration_set_name
  event_destination_name = "${local.name_prefix}-email-failures"

  event_destination {
    enabled = true
    matching_event_types = [
      "BOUNCE",
      "COMPLAINT",
      "DELIVERY_DELAY",
      "REJECT",
    ]

    sns_destination {
      topic_arn = aws_sns_topic.email_failures.arn
    }
  }

  depends_on = [aws_sns_topic_policy.email_failures]
}
