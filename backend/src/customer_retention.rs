use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct CleanupSummary {
    pub customers_anonymized: u64,
    pub addresses_removed: u64,
    pub accounts_removed: u64,
    pub sessions_removed: u64,
    pub login_rate_limits_removed: u64,
    pub account_tokens_removed: u64,
}

pub async fn cleanup_expired_customer_data(
    pool: &PgPool,
    batch_size: i64,
    revoked_session_retention_days: i32,
) -> Result<CleanupSummary, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let mut summary = CleanupSummary::default();

    summary.sessions_removed += sqlx::query(
        r#"
        DELETE FROM customer_sessions
        WHERE expires_at <= now()
           OR revoked_at <= now() - make_interval(days => $1)
        "#,
    )
    .bind(revoked_session_retention_days)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    summary.login_rate_limits_removed += sqlx::query(
        "DELETE FROM auth_login_rate_limits WHERE auth_scope = 'customer' AND updated_at < now() - interval '24 hours'",
    )
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    summary.account_tokens_removed += sqlx::query(
        r#"
        DELETE FROM customer_account_tokens
        WHERE expires_at <= now()
           OR used_at <= now() - make_interval(days => $1)
        "#,
    )
    .bind(revoked_session_retention_days)
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    let expired = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM customers
        WHERE anonymized_at IS NULL
          AND retention_expires_at <= now()
        ORDER BY retention_expires_at, id
        LIMIT $1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(batch_size)
    .fetch_all(&mut *transaction)
    .await?;

    for customer_id in expired {
        summary.sessions_removed += sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM customer_sessions WHERE customer_id = $1",
        )
        .bind(customer_id)
        .fetch_one(&mut *transaction)
        .await? as u64;
        summary.addresses_removed +=
            sqlx::query("DELETE FROM customer_addresses WHERE customer_id = $1")
                .bind(customer_id)
                .execute(&mut *transaction)
                .await?
                .rows_affected();
        summary.accounts_removed +=
            sqlx::query("DELETE FROM customer_accounts WHERE customer_id = $1")
                .bind(customer_id)
                .execute(&mut *transaction)
                .await?
                .rows_affected();

        sqlx::query(
            r#"
            UPDATE customers
            SET email = $2,
                first_name = 'Anonymized',
                last_name = 'Customer',
                phone = '',
                anonymized_at = now(),
                updated_at = now()
            WHERE id = $1
              AND anonymized_at IS NULL
            "#,
        )
        .bind(customer_id)
        .bind("anonymized@knitprint.invalid")
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO audit_log (action, entity_type, entity_id, reason)
            VALUES ('customer.retention_anonymize', 'customer', $1, $2)
            "#,
        )
        .bind(customer_id.to_string())
        .bind("Retention deadline reached; personal data and authentication records removed")
        .execute(&mut *transaction)
        .await?;
        summary.customers_anonymized += 1;
    }

    transaction.commit().await?;
    Ok(summary)
}
