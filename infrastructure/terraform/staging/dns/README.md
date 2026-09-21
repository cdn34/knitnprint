# Staging DNS and email prerequisites

This stack requests AWS resources whose validation records must be published at
the external DNS provider. It does not create or modify Route 53 zones or DNS
records.

It manages:

- an ACM certificate in `eu-west-1` for `staging.knitnprint.com`;
- an ACM certificate in `us-east-1` for `admin.staging.knitnprint.com`;
- an SES Easy DKIM identity in `eu-west-1` for `staging.knitnprint.com`;
- the custom MAIL FROM domain `bounce.staging.knitnprint.com` with reject-on-MX-failure behavior;
- the `knitnprint-staging-transactional` SES configuration set;
- the Ireland account suppression reasons `BOUNCE` and `COMPLAINT`;
- the `knitnprint-staging-email-failures` standard SNS topic and SES event destination.

The SNS destination records bounce, complaint, reject, and delivery-delay
events. It intentionally has no subscriber yet; the observability stack will
attach the final operational alert destination.

Both ACM certificates are explicitly non-exportable for use with AWS-integrated
services. SES configuration-set reputation metrics remain disabled because they
create separately billed CloudWatch metrics; the SNS event destination provides
the failure-event stream needed at this stage.

After apply, use `terraform output` to copy the ACM, Easy DKIM, MAIL FROM MX,
and SPF TXT records into Namecheap. Namecheap may automatically append the zone
name, so confirm whether its Host field expects a relative label before saving.
Do not create `aws_acm_certificate_validation` resources here: with external DNS
they would block apply while waiting for records that Terraform cannot publish.

The Easy DKIM output uses the current SES `SigningHostedZone` observed in
`eu-west-1`, `dkim.amazonses.com`. Verify that value with `GetEmailIdentity`
immediately after creation before publishing the records.

AWS references:

- [Easy DKIM](https://docs.aws.amazon.com/ses/latest/dg/send-email-authentication-dkim-easy.html)
- [Custom MAIL FROM records](https://docs.aws.amazon.com/ses/latest/dg/mail-from.html)
- [SNS event destinations](https://docs.aws.amazon.com/ses/latest/dg/event-publishing-add-event-destination-sns.html)
