# Permission boundary for staging only; production gets a distinct deployer.
data "aws_iam_policy_document" "staging_deployer" {
  statement {
    sid = "ReadDeploymentIdentity"

    actions = [
      "iam:GetAccountSummary",
      "iam:GetPolicy",
      "iam:GetPolicyVersion",
      "iam:GetRole",
      "iam:GetRolePolicy",
      "iam:ListAttachedRolePolicies",
      "iam:ListPolicies",
      "iam:ListPolicyVersions",
      "iam:ListRolePolicies",
      "iam:ListRoles",
      "servicequotas:GetServiceQuota",
      "servicequotas:ListServiceQuotas",
      "sts:GetCallerIdentity",
      "tag:GetResources",
      "tag:GetTagKeys",
      "tag:GetTagValues",
    ]

    resources = ["*"]
  }

  statement {
    sid = "ManageKnitNPrintRoles"

    actions = [
      "iam:AttachRolePolicy",
      "iam:CreateRole",
      "iam:DeleteRole",
      "iam:DeleteRolePolicy",
      "iam:DetachRolePolicy",
      "iam:PutRolePolicy",
      "iam:TagRole",
      "iam:UntagRole",
      "iam:UpdateAssumeRolePolicy",
      "iam:UpdateRole",
      "iam:UpdateRoleDescription",
    ]

    resources = [
      "arn:${local.partition}:iam::${local.account_id}:role/${local.name_prefix}-*",
    ]
  }

  statement {
    sid = "ManageKnitNPrintPolicies"

    actions = [
      "iam:CreatePolicy",
      "iam:CreatePolicyVersion",
      "iam:DeletePolicy",
      "iam:DeletePolicyVersion",
      "iam:SetDefaultPolicyVersion",
      "iam:TagPolicy",
      "iam:UntagPolicy",
    ]

    resources = [
      "arn:${local.partition}:iam::${local.account_id}:policy/${local.name_prefix}-*",
    ]
  }

  statement {
    sid = "PassKnitNPrintWorkloadRoles"

    actions = ["iam:PassRole"]

    resources = [
      "arn:${local.partition}:iam::${local.account_id}:role/${local.name_prefix}-*",
    ]

    condition {
      test     = "StringEquals"
      variable = "iam:PassedToService"
      values = [
        "ecs-tasks.amazonaws.com",
        "events.amazonaws.com",
        "rds.amazonaws.com",
        "scheduler.amazonaws.com",
      ]
    }
  }

  statement {
    sid = "ManageRequiredServiceLinkedRoles"

    actions = [
      "iam:CreateServiceLinkedRole",
      "iam:DeleteServiceLinkedRole",
      "iam:GetServiceLinkedRoleDeletionStatus",
    ]

    resources = ["*"]

    condition {
      test     = "StringEquals"
      variable = "iam:AWSServiceName"
      values = [
        "application-autoscaling.amazonaws.com",
        "autoscaling.amazonaws.com",
        "ecs.amazonaws.com",
        "elasticloadbalancing.amazonaws.com",
        "rds.amazonaws.com",
      ]
    }
  }

  statement {
    sid = "ManageIrelandStagingInfrastructure"

    actions = [
      "acm:*",
      "application-autoscaling:*",
      "autoscaling:*",
      "cloudwatch:*",
      "ec2:*",
      "ecr:*",
      "ecs:*",
      "elasticloadbalancing:*",
      "events:*",
      "logs:*",
      "rds:*",
      "scheduler:*",
      "secretsmanager:*",
      "ses:*",
      "sns:*",
      "sqs:*",
      "wafv2:*",
    ]

    resources = ["*"]

    condition {
      test     = "StringEquals"
      variable = "aws:RequestedRegion"
      values   = [var.aws_region]
    }
  }

  statement {
    sid = "ManageCloudFrontCertificateAndWaf"

    actions = [
      "acm:*",
      "wafv2:*",
    ]

    resources = ["*"]

    condition {
      test     = "StringEquals"
      variable = "aws:RequestedRegion"
      values   = [var.identity_region]
    }
  }

  statement {
    sid       = "ManageCloudFront"
    actions   = ["cloudfront:*"]
    resources = ["*"]
  }

  statement {
    sid = "ListS3Buckets"

    actions = [
      "s3:GetAccountPublicAccessBlock",
      "s3:ListAllMyBuckets",
    ]

    resources = ["*"]
  }

  statement {
    sid = "ManageKnitNPrintBuckets"

    actions = ["s3:*"]

    resources = [
      "arn:${local.partition}:s3:::${local.name_prefix}-*",
      "arn:${local.partition}:s3:::${local.name_prefix}-*/*",
    ]
  }
}

resource "aws_ssoadmin_permission_set" "staging_deployer" {
  provider = aws.identity

  instance_arn     = local.identity_instance_arn
  name             = var.deployer_permission_set_name
  description      = "Terraform deployment access scoped to KnitNPrint staging services and Regions."
  session_duration = "PT4H"

  tags = local.common_tags
}

resource "aws_ssoadmin_permission_set_inline_policy" "staging_deployer" {
  provider = aws.identity

  inline_policy      = data.aws_iam_policy_document.staging_deployer.json
  instance_arn       = local.identity_instance_arn
  permission_set_arn = aws_ssoadmin_permission_set.staging_deployer.arn
}

resource "aws_ssoadmin_account_assignment" "staging_administrators" {
  provider = aws.identity

  instance_arn       = local.identity_instance_arn
  permission_set_arn = aws_ssoadmin_permission_set.staging_deployer.arn

  principal_id   = data.aws_identitystore_group.administrators.group_id
  principal_type = "GROUP"

  target_id   = local.account_id
  target_type = "AWS_ACCOUNT"

  depends_on = [aws_ssoadmin_permission_set_inline_policy.staging_deployer]
}
