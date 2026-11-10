use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    customer_auth::{hash_token, new_session_token},
    email::{AccountEmailKind, EmailService},
    notifications::{ClaimedNotification, DeliveryOutcome},
};

#[derive(FromRow)]
struct Account {
    email: String,
    first_name: String,
    verified: bool,
}

#[derive(FromRow)]
struct AccountToken {
    token_kind: String,
    target_email: Option<String>,
    used: bool,
    request_current: bool,
}

enum Prepared {
    Ready {
        to: String,
        first_name: String,
        kind: AccountEmailKind,
        token: String,
    },
    Suppressed(&'static str),
    Stale,
}

pub(crate) async fn deliver(
    pool: &PgPool,
    email: &EmailService,
    job: &ClaimedNotification,
) -> DeliveryOutcome {
    match prepare(pool, job).await {
        Ok(Prepared::Ready {
            to,
            first_name,
            kind,
            token,
        }) => {
            match email
                .send_account_action(&to, &first_name, kind, &token)
                .await
            {
                Ok(()) => DeliveryOutcome::Sent,
                Err(error) => {
                    let message = safe_delivery_error(&error);
                    tracing::warn!(notification_id = %job.id, email_kind = kind.as_str(), %message, "account email delivery failed; notification remains retryable");
                    DeliveryOutcome::Failed(message.into())
                }
            }
        }
        Ok(Prepared::Suppressed(reason)) => DeliveryOutcome::Suppressed(reason),
        Ok(Prepared::Stale) => DeliveryOutcome::Stale,
        Err(_) => {
            DeliveryOutcome::Failed("Account email could not be prepared. Retry delivery.".into())
        }
    }
}

async fn prepare(pool: &PgPool, job: &ClaimedNotification) -> Result<Prepared, sqlx::Error> {
    let Some(kind) = AccountEmailKind::from_notification_kind(&job.kind) else {
        return Ok(Prepared::Suppressed("Unsupported account email type."));
    };
    let Some(token_id) = job.account_token_id else {
        return Ok(Prepared::Suppressed(
            "Account request is no longer available.",
        ));
    };
    let mut tx = pool.begin().await?;
    // Fence token rotation against a newer delivery claim. A stale worker must
    // never invalidate a link issued by the current worker or administrator.
    let current = sqlx::query_scalar::<_, Uuid>("SELECT id FROM notification_jobs WHERE id=$1 AND status='processing' AND attempt_count=$2 FOR UPDATE")
        .bind(job.id).bind(job.attempt_count).fetch_optional(&mut *tx).await?;
    if current.is_none() {
        return Ok(Prepared::Stale);
    }
    let customer_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT customer_id FROM customer_account_tokens WHERE id=$1",
    )
    .bind(token_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(customer_id) = customer_id else {
        return Ok(Prepared::Suppressed(
            "Account request is no longer available.",
        ));
    };
    // Account requests also lock account/customer before updating tokens.
    let account = sqlx::query_as::<_, Account>(
        r#"
        SELECT customer.email::text, customer.first_name,
               account.email_verified_at IS NOT NULL AS verified
        FROM customer_accounts account JOIN customers customer ON customer.id=account.customer_id
        WHERE customer.id=$1 AND account.disabled_at IS NULL
          AND customer.customer_type='registered' AND customer.anonymized_at IS NULL
          AND customer.retention_expires_at > now()
        FOR UPDATE OF account, customer
    "#,
    )
    .bind(customer_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(account) = account else {
        return Ok(Prepared::Suppressed("Account is no longer active."));
    };
    let token = sqlx::query_as::<_, AccountToken>(
        r#"
        SELECT token_kind, target_email::text, used_at IS NOT NULL AS used,
               created_at + CASE WHEN token_kind='password_reset' THEN interval '1 hour'
                                 ELSE interval '24 hours' END > now() AS request_current
        FROM customer_account_tokens WHERE id=$1 AND customer_id=$2 FOR UPDATE NOWAIT
    "#,
    )
    .bind(token_id)
    .bind(customer_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(token) = token else {
        return Ok(Prepared::Suppressed(
            "Account request is no longer available.",
        ));
    };
    if token.used || token.token_kind != job.kind {
        return Ok(Prepared::Suppressed(
            "Account request was completed or replaced by a newer request.",
        ));
    }
    match kind {
        AccountEmailKind::Verification => {
            if account.verified || !account.email.eq_ignore_ascii_case(&job.recipient_email) {
                return Ok(Prepared::Suppressed(
                    "Email address is already verified or has changed.",
                ));
            }
        }
        AccountEmailKind::PasswordReset => {
            if !token.request_current || !account.email.eq_ignore_ascii_case(&job.recipient_email) {
                return Ok(Prepared::Suppressed(
                    "Password reset request expired or the email address changed.",
                ));
            }
        }
        AccountEmailKind::EmailChange => {
            if !account.verified
                || !token.request_current
                || !token
                    .target_email
                    .as_ref()
                    .is_some_and(|target| target.eq_ignore_ascii_case(&job.recipient_email))
                || account.email.eq_ignore_ascii_case(&job.recipient_email)
            {
                return Ok(Prepared::Suppressed(
                    "Email change request expired or is no longer current.",
                ));
            }
            let in_use = sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM customers WHERE email=$1 AND id<>$2 AND customer_type='registered' AND anonymized_at IS NULL)")
                .bind(&job.recipient_email).bind(customer_id).fetch_one(&mut *tx).await?;
            if in_use {
                return Ok(Prepared::Suppressed(
                    "The requested email address is no longer available.",
                ));
            }
        }
    }
    let secret = new_session_token();
    let token_hash = hash_token(&secret);
    sqlx::query("UPDATE customer_account_tokens SET token_hash=$2, expires_at=now()+$3::interval WHERE id=$1")
        .bind(token_id).bind(token_hash.as_slice()).bind(kind.expiry_interval()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Prepared::Ready {
        to: job.recipient_email.clone(),
        first_name: account.first_name,
        kind,
        token: secret,
    })
}

fn safe_delivery_error(error: &str) -> &'static str {
    // Provider debug responses must not put action links, credentials, or email
    // content into the persistent outbox or its administrator-facing errors.
    if error.contains("ExpiredToken")
        || error.contains("UnauthorizedException")
        || error.contains("session token")
    {
        "AWS session expired or invalid. Sign in to AWS and retry this notification."
    } else if error.contains("not verified") || error.contains("MessageRejected") {
        "The email provider rejected this message. Check sender and recipient verification."
    } else if error.contains("not configured") {
        "Email delivery is not configured. Configure the email provider and retry."
    } else {
        "Account email delivery failed. Check the email provider and retry this notification."
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn provider_responses_cannot_leak_action_links_or_secrets_into_the_outbox() {
        let secret = "1".repeat(64);
        let response = format!(
            "SES rejected body https://store.example/account?verify={secret} AWS_ACCESS_KEY_ID=secret"
        );
        let visible = super::safe_delivery_error(&response);
        assert!(!visible.contains(&secret));
        assert!(!visible.contains("https://"));
        assert!(!visible.contains("AWS_ACCESS_KEY_ID"));
        assert!(
            super::safe_delivery_error("UnauthorizedException: Session token not found or invalid")
                .contains("Sign in to AWS")
        );
    }
}
