use std::{env, str::FromStr};

use knitprint_api::customer_retention::{CleanupSummary, cleanup_expired_customer_data};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use uuid::Uuid;

#[tokio::test]
async fn expired_customer_cleanup_is_bounded_irreversible_and_repeat_safe() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("customer_cleanup_test_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("test database should be available");
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&admin)
        .await
        .expect("test schema should be created");
    let pool = isolated_pool(&database_url, &schema).await;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations should run in the isolated schema");

    let expired_guest = Uuid::now_v7();
    let expired_registered = Uuid::now_v7();
    let active_registered = Uuid::now_v7();
    insert_customer(
        &pool,
        expired_guest,
        "guest",
        "expired-guest@example.test",
        true,
    )
    .await;
    insert_customer(
        &pool,
        expired_registered,
        "registered",
        "expired-account@example.test",
        true,
    )
    .await;
    insert_customer(
        &pool,
        active_registered,
        "registered",
        "active-account@example.test",
        false,
    )
    .await;
    sqlx::query(
        "UPDATE customers SET retention_expires_at = now() - interval '2 days' WHERE id = $1",
    )
    .bind(expired_guest)
    .execute(&pool)
    .await
    .expect("guest should be the first retention candidate");

    let expired_guest_address = insert_address(&pool, expired_guest, "Old Guest").await;
    insert_address(&pool, expired_registered, "Old Account").await;
    insert_address(&pool, active_registered, "Active Account").await;
    sqlx::query(
        "INSERT INTO guest_customer_requests (idempotency_hash, customer_id, address_id) VALUES ($1, $2, $3)",
    )
    .bind(vec![7_u8; 32])
    .bind(expired_guest)
    .bind(expired_guest_address)
    .execute(&pool)
    .await
    .expect("guest idempotency fixture should be inserted");

    insert_account(&pool, expired_registered).await;
    insert_account(&pool, active_registered).await;
    sqlx::query(
        r#"
        INSERT INTO customer_account_tokens (
            id, customer_id, token_kind, token_hash, created_at, expires_at, used_at
        ) VALUES
            ($1, $4, 'email_verification', $5, now() - interval '2 days', now() - interval '1 day', NULL),
            ($2, $4, 'password_reset', $6, now() - interval '10 days', now() + interval '1 day', now() - interval '8 days'),
            ($3, $4, 'email_verification', $7, now(), now() + interval '1 day', NULL)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(Uuid::now_v7())
    .bind(Uuid::now_v7())
    .bind(active_registered)
    .bind(vec![11_u8; 32])
    .bind(vec![12_u8; 32])
    .bind(vec![13_u8; 32])
    .execute(&pool)
    .await
    .expect("account-token cleanup fixtures should be inserted");
    insert_session(&pool, expired_registered, "active").await;
    insert_session(&pool, active_registered, "active").await;
    insert_session(&pool, active_registered, "expired").await;
    insert_session(&pool, active_registered, "revoked").await;
    insert_login_attempt(&pool, false).await;
    insert_login_attempt(&pool, true).await;
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_customer_id, action, entity_type, entity_id)
        VALUES ($1, 'customer.login', 'customer_session', $2)
        "#,
    )
    .bind(expired_registered)
    .bind(Uuid::now_v7().to_string())
    .execute(&pool)
    .await
    .expect("historical audit fixture should be inserted");

    let first_batch = cleanup_expired_customer_data(&pool, 1, 7)
        .await
        .expect("cleanup should succeed");
    assert_eq!(
        first_batch,
        CleanupSummary {
            customers_anonymized: 1,
            addresses_removed: 1,
            accounts_removed: 0,
            sessions_removed: 2,
            login_rate_limits_removed: 1,
            account_tokens_removed: 2,
        }
    );
    let second_batch = cleanup_expired_customer_data(&pool, 1, 7)
        .await
        .expect("the next bounded cleanup batch should succeed");
    assert_eq!(
        second_batch,
        CleanupSummary {
            customers_anonymized: 1,
            addresses_removed: 1,
            accounts_removed: 1,
            sessions_removed: 1,
            login_rate_limits_removed: 0,
            account_tokens_removed: 0,
        }
    );
    let active_account_tokens: i64 =
        sqlx::query_scalar("SELECT count(*) FROM customer_account_tokens WHERE customer_id = $1")
            .bind(active_registered)
            .fetch_one(&pool)
            .await
            .expect("active account tokens should be inspectable");
    assert_eq!(active_account_tokens, 1);

    let anonymized: Vec<(Uuid, String, String, String, String, bool)> = sqlx::query_as(
        r#"
        SELECT id, email::text, first_name, last_name, phone, anonymized_at IS NOT NULL
        FROM customers
        WHERE id = ANY($1)
        ORDER BY id
        "#,
    )
    .bind(vec![expired_guest, expired_registered])
    .fetch_all(&pool)
    .await
    .expect("anonymized customers should remain as non-personal commercial identities");
    assert_eq!(anonymized.len(), 2);
    for (_, email, first_name, last_name, phone, marked) in anonymized {
        assert_eq!(email, "anonymized@knitprint.invalid");
        assert_eq!(first_name, "Anonymized");
        assert_eq!(last_name, "Customer");
        assert!(phone.is_empty());
        assert!(marked);
    }
    let removed_addresses: i64 =
        sqlx::query_scalar("SELECT count(*) FROM customer_addresses WHERE customer_id = ANY($1)")
            .bind(vec![expired_guest, expired_registered])
            .fetch_one(&pool)
            .await
            .expect("address cleanup should be inspectable");
    assert_eq!(removed_addresses, 0);
    let removed_accounts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM customer_accounts WHERE customer_id = $1")
            .bind(expired_registered)
            .fetch_one(&pool)
            .await
            .expect("account cleanup should be inspectable");
    assert_eq!(removed_accounts, 0);
    let guest_requests: i64 = sqlx::query_scalar("SELECT count(*) FROM guest_customer_requests")
        .fetch_one(&pool)
        .await
        .expect("guest request cleanup should be inspectable");
    assert_eq!(guest_requests, 0);

    let active: (String, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT customer.email::text, address.recipient_name,
               (SELECT count(*) FROM customer_accounts WHERE customer_id = customer.id),
               (SELECT count(*) FROM customer_sessions WHERE customer_id = customer.id)
        FROM customers customer
        JOIN customer_addresses address ON address.customer_id = customer.id
        WHERE customer.id = $1
        "#,
    )
    .bind(active_registered)
    .fetch_one(&pool)
    .await
    .expect("unexpired customer data should remain");
    assert_eq!(
        active,
        (
            "active-account@example.test".into(),
            "Active Account".into(),
            1,
            1,
        )
    );
    let retention_audits: Vec<(String, String, Option<Uuid>)> = sqlx::query_as(
        r#"
        SELECT entity_id, reason, actor_customer_id
        FROM audit_log
        WHERE action = 'customer.retention_anonymize'
        ORDER BY entity_id
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("cleanup audits should be readable");
    assert_eq!(retention_audits.len(), 2);
    assert!(retention_audits.iter().all(|(_, reason, actor)| {
        reason == "Retention deadline reached; personal data and authentication records removed"
            && actor.is_none()
    }));
    let historical_actor: Option<Uuid> = sqlx::query_scalar(
        "SELECT actor_customer_id FROM audit_log WHERE action = 'customer.login'",
    )
    .fetch_one(&pool)
    .await
    .expect("historical audit identity should be retained without contact data");
    assert_eq!(historical_actor, Some(expired_registered));

    assert_eq!(
        cleanup_expired_customer_data(&pool, 100, 7)
            .await
            .expect("repeated cleanup should succeed"),
        CleanupSummary::default()
    );

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}

async fn insert_customer(pool: &PgPool, id: Uuid, kind: &str, email: &str, expired: bool) {
    let (created_at, retention_expires_at) = if expired {
        ("now() - interval '3 days'", "now() - interval '1 day'")
    } else {
        ("now()", "now() + interval '24 months'")
    };
    sqlx::query(&format!(
        r#"
        INSERT INTO customers (
            id, customer_type, email, first_name, last_name, phone,
            created_at, retention_expires_at
        )
        VALUES ($1, $2, $3, 'Personal', 'Details', '+351 210 000 000', {created_at}, {retention_expires_at})
        "#,
    ))
    .bind(id)
    .bind(kind)
    .bind(email)
    .execute(pool)
    .await
    .expect("customer fixture should be inserted");
}

async fn insert_address(pool: &PgPool, customer_id: Uuid, recipient: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO customer_addresses (
            id, customer_id, recipient_name, line1, city, postal_code, country_code, phone
        )
        VALUES ($1, $2, $3, '12 Private Lane', 'Lisbon', '1000-001', 'PT', '+351 210 000 000')
        "#,
    )
    .bind(id)
    .bind(customer_id)
    .bind(recipient)
    .execute(pool)
    .await
    .expect("address fixture should be inserted");
    id
}

async fn insert_account(pool: &PgPool, customer_id: Uuid) {
    sqlx::query(
        "INSERT INTO customer_accounts (customer_id, password_hash) VALUES ($1, '$argon2id$fixture')",
    )
    .bind(customer_id)
    .execute(pool)
    .await
    .expect("account fixture should be inserted");
}

async fn insert_session(pool: &PgPool, customer_id: Uuid, state: &str) {
    let id = Uuid::now_v7();
    let token_hash = id.as_bytes().repeat(2);
    let (created_at, expires_at, revoked_at) = match state {
        "expired" => (
            "now() - interval '2 days'",
            "now() - interval '1 day'",
            "NULL",
        ),
        "revoked" => (
            "now() - interval '10 days'",
            "now() + interval '20 days'",
            "now() - interval '8 days'",
        ),
        _ => ("now()", "now() + interval '30 days'", "NULL"),
    };
    sqlx::query(&format!(
        r#"
        INSERT INTO customer_sessions (
            id, customer_id, token_hash, created_at, expires_at, last_seen_at, revoked_at
        )
        VALUES ($1, $2, $3, {created_at}, {expires_at}, {created_at}, {revoked_at})
        "#,
    ))
    .bind(id)
    .bind(customer_id)
    .bind(token_hash)
    .execute(pool)
    .await
    .expect("session fixture should be inserted");
}

async fn insert_login_attempt(pool: &PgPool, stale: bool) {
    let updated_at = if stale {
        "now() - interval '2 days'"
    } else {
        "now()"
    };
    let key = Uuid::now_v7().as_bytes().repeat(2);
    sqlx::query(&format!(
        "INSERT INTO auth_login_rate_limits (auth_scope, dimension, key_hash, event_count, updated_at) VALUES ('customer', 'account', $1, 1, {updated_at})",
    ))
    .bind(key)
    .execute(pool)
    .await
    .expect("login-attempt fixture should be inserted");
}

async fn isolated_pool(database_url: &str, schema: &str) -> PgPool {
    let options = PgConnectOptions::from_str(database_url).expect("DATABASE_URL should be valid");
    let search_path = format!(r#"SET search_path TO "{schema}", public"#);
    PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query(&search_path).execute(connection).await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .expect("isolated test pool should connect")
}
