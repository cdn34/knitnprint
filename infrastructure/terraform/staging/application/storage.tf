resource "aws_s3_bucket" "private" {
  for_each = local.private_buckets

  bucket        = each.value
  force_destroy = false

  tags = {
    Purpose = each.key
  }
}

resource "aws_s3_bucket_ownership_controls" "private" {
  for_each = aws_s3_bucket.private

  bucket = each.value.id

  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

resource "aws_s3_bucket_public_access_block" "private" {
  for_each = aws_s3_bucket.private

  bucket = each.value.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_server_side_encryption_configuration" "private" {
  for_each = aws_s3_bucket.private

  bucket = each.value.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_versioning" "private" {
  for_each = aws_s3_bucket.private

  bucket = each.value.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "private" {
  for_each = aws_s3_bucket.private

  bucket = each.value.id

  rule {
    id     = "staging-storage-retention"
    status = "Enabled"

    filter {}

    abort_incomplete_multipart_upload {
      days_after_initiation = 7
    }

    noncurrent_version_expiration {
      noncurrent_days = 30
    }
  }

  depends_on = [aws_s3_bucket_versioning.private]
}

data "aws_iam_policy_document" "private_bucket" {
  for_each = aws_s3_bucket.private

  statement {
    sid    = "DenyInsecureTransport"
    effect = "Deny"

    principals {
      type        = "*"
      identifiers = ["*"]
    }

    actions = ["s3:*"]

    resources = [
      each.value.arn,
      "${each.value.arn}/*",
    ]

    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"
      values   = ["false"]
    }
  }

}

data "aws_iam_policy_document" "admin_assets_bucket" {
  source_policy_documents = [data.aws_iam_policy_document.private_bucket["admin_assets"].json]

  statement {
    sid    = "AllowAdminCloudFrontRead"
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["cloudfront.amazonaws.com"]
    }

    actions   = ["s3:GetObject"]
    resources = ["${aws_s3_bucket.private["admin_assets"].arn}/*"]

    condition {
      test     = "StringEquals"
      variable = "AWS:SourceArn"
      values   = [aws_cloudfront_distribution.admin.arn]
    }
  }
}

resource "aws_s3_bucket_policy" "private" {
  for_each = aws_s3_bucket.private

  bucket = each.value.id
  policy = each.key == "admin_assets" ? data.aws_iam_policy_document.admin_assets_bucket.json : data.aws_iam_policy_document.private_bucket[each.key].json

  depends_on = [aws_s3_bucket_public_access_block.private]
}

resource "aws_s3_bucket_cors_configuration" "media" {
  bucket = aws_s3_bucket.private["media"].id

  cors_rule {
    id = "signed-browser-requests"

    allowed_headers = ["*"]
    allowed_methods = ["GET", "HEAD", "PUT"]
    allowed_origins = [
      "https://staging.knitnprint.com",
      "https://admin.staging.knitnprint.com",
    ]
    expose_headers  = ["ETag"]
    max_age_seconds = 300
  }
}
