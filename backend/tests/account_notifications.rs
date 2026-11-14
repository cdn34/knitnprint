use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitnprint_api::{
    AppState, app, auth::SESSION_COOKIE, email::EmailService, notifications::deliver_due,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

async fn request(
    router: &axum::Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = if let Some(body) = body {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn latest(router: &axum::Router, address: &str, kind: &str) -> Value {
    let response = request(
        router,
        "GET",
        &format!("/api/development/emails/latest?to={address}&kind={kind}"),
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    body(response).await
}

async fn register(router: &axum::Router, address: &str) -> Uuid {
    let response = request(router, "POST", "/api/account/register", None, Some(json!({
        "email":address, "password":"account-notification-passphrase", "first_name":"Account", "last_name":"Tester"
    }))).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    Uuid::parse_str(body(response).await["id"].as_str().unwrap()).unwrap()
}

async fn owner(pool: &PgPool, role: &str) -> String {
    let id = Uuid::now_v7();
    let token = format!("notification-session-{id}");
    sqlx::query("INSERT INTO staff_users(id,email,display_name,password_hash,role) VALUES($1,$2,'Notification staff','unused',$3)")
        .bind(id).bind(format!("{id}@example.test")).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO staff_sessions(id,staff_user_id,token_hash,expires_at) VALUES($1,$2,$3,now()+interval '1 day')")
        .bind(Uuid::now_v7()).bind(id).bind(Sha256::digest(token.as_bytes()).as_slice()).execute(pool).await.unwrap();
    if role == "staff" {
        sqlx::query("INSERT INTO staff_capabilities(staff_user_id,capability_name) VALUES($1,'notifications.read')").bind(id).execute(pool).await.unwrap();
    }
    format!("{SESSION_COOKIE}={token}")
}

async fn job(pool: &PgPool, recipient: &str, kind: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM notification_jobs WHERE recipient_email=$1 AND kind=$2 ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(recipient).bind(kind).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn account_email_failures_are_durable_searchable_and_safely_retryable() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("account_notifications_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    let options = PgConnectOptions::from_str(&url)
        .unwrap()
        .options([("search_path", format!("{schema},public"))]);
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    // Disabled delivery models the real failure path without contacting SES.
    let failed = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });
    let mailbox = EmailService::development("http://127.0.0.1:3000");
    let healthy = app(AppState {
        database: Some(pool.clone()),
        email: mailbox.clone(),
        ..AppState::default()
    });
    let owner = owner(&pool, "owner").await;
    let reader = self::owner(&pool, "staff").await;
    let address = "activation@example.test";
    let customer_id = register(&failed, address).await;
    let id = job(&pool, address, "email_verification").await;
    let (status,count,error,token_id):(String,i32,String,Uuid) = sqlx::query_as("SELECT status,attempt_count,last_error,account_token_id FROM notification_jobs WHERE id=$1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!((status.as_str(), count), ("pending", 1));
    assert!(error.contains("not configured"));
    let old_hash: Vec<u8> =
        sqlx::query_scalar("SELECT token_hash FROM customer_account_tokens WHERE id=$1")
            .bind(token_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let path = format!("/api/admin/notifications/{id}/send");
    let list = request(
        &healthy,
        "GET",
        "/api/admin/notifications?q=activation%40example.test&kind=email_verification",
        Some(&reader),
        None,
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let listing = body(list).await;
    assert_eq!(listing["total"], 1);
    assert_eq!(listing["items"][0]["can_send"], true);
    assert!(listing["items"][0]["order_id"].is_null());
    assert!(!listing.to_string().contains("token_hash"));
    assert!(!listing.to_string().contains("account_token_id"));
    assert_eq!(
        request(
            &healthy,
            "POST",
            &path,
            Some(&reader),
            Some(json!({"reason":"Retry account verification"}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE customer_account_tokens SET created_at=now()-interval '2 days',expires_at=now()-interval '1 day' WHERE id=$1")
        .bind(token_id).execute(&pool).await.unwrap();

    let reason = json!({"reason":"Customer reported a failed activation email"});
    let (first, second) = tokio::join!(
        request(&healthy, "POST", &path, Some(&owner), Some(reason.clone())),
        request(&healthy, "POST", &path, Some(&owner), Some(reason.clone()))
    );
    let statuses = [first.status(), second.status()];
    assert_eq!(
        statuses
            .iter()
            .filter(|&&status| status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|&&status| status == StatusCode::CONFLICT)
            .count(),
        1
    );
    let message = latest(&healthy, address, "email_verification").await;
    let first_secret = message["action_url"]
        .as_str()
        .unwrap()
        .split("verify=")
        .nth(1)
        .unwrap()
        .to_owned();
    let new_hash: Vec<u8> =
        sqlx::query_scalar("SELECT token_hash FROM customer_account_tokens WHERE id=$1")
            .bind(token_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(old_hash, new_hash);
    assert_eq!(new_hash, Sha256::digest(first_secret.as_bytes()).as_slice());
    let persisted: String =
        sqlx::query_scalar("SELECT row_to_json(job)::text FROM notification_jobs job WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!persisted.contains(&first_secret));
    assert_eq!(
        request(&healthy, "POST", &path, Some(&owner), Some(reason.clone()))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    // Model a lost completion after provider acceptance. Retrying rotates the link.
    sqlx::query("UPDATE notification_jobs SET status='pending',sent_at=NULL WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let retried =
        body(request(&healthy, "POST", &path, Some(&owner), Some(reason.clone())).await).await;
    assert_eq!(retried["status"], "sent");
    assert_eq!(retried["attempt_count"], 3);
    let second_secret = latest(&healthy, address, "email_verification").await["action_url"]
        .as_str()
        .unwrap()
        .split("verify=")
        .nth(1)
        .unwrap()
        .to_owned();
    assert_ne!(first_secret, second_secret);
    assert_eq!(
        request(
            &healthy,
            "POST",
            "/api/account/verification/confirm",
            None,
            Some(json!({"token":first_secret}))
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        request(
            &healthy,
            "POST",
            "/api/account/verification/confirm",
            None,
            Some(json!({"token":second_secret}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    sqlx::query("UPDATE notification_jobs SET status='pending',sent_at=NULL WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let obsolete =
        body(request(&healthy, "POST", &path, Some(&owner), Some(reason.clone())).await).await;
    assert_eq!(obsolete["status"], "suppressed");
    let outcomes:Vec<String>=sqlx::query_scalar("SELECT outcome FROM notification_attempts WHERE notification_job_id=$1 ORDER BY attempt_number").bind(id).fetch_all(&pool).await.unwrap();
    assert_eq!(outcomes, ["failed", "sent", "sent", "suppressed"]);

    // A process/credential outage is also recovered by the ordinary worker.
    register(&failed, "worker@example.test").await;
    let worker_job = job(&pool, "worker@example.test", "email_verification").await;
    sqlx::query("UPDATE notification_jobs SET next_attempt_at=now() WHERE id=$1")
        .bind(worker_job)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(deliver_due(&pool, &mailbox, 10).await.unwrap().sent, 1);
    let jobs_before: i64 = sqlx::query_scalar("SELECT count(*) FROM notification_jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &healthy,
            "POST",
            "/api/account/verification/request",
            None,
            Some(json!({"email":"worker@example.test"}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notification_jobs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        jobs_before
    );

    // A newer customer request invalidates an older pending email, rather than
    // letting the worker send two different activation links.
    sqlx::query(
        "UPDATE customer_account_tokens SET created_at=now()-interval '6 minutes' WHERE id=$1",
    )
    .bind(worker_job)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE notification_jobs SET status='pending',sent_at=NULL WHERE id=$1")
        .bind(worker_job)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &failed,
            "POST",
            "/api/account/verification/request",
            None,
            Some(json!({"email":"worker@example.test"}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let replacement = job(&pool, "worker@example.test", "email_verification").await;
    assert_ne!(worker_job, replacement);
    sqlx::query("UPDATE notification_jobs SET next_attempt_at=now() WHERE recipient_email='worker@example.test' AND status='pending'")
        .execute(&pool).await.unwrap();
    assert_eq!(deliver_due(&pool, &mailbox, 10).await.unwrap().sent, 1);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM notification_jobs WHERE id=$1")
            .bind(worker_job)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "suppressed"
    );

    // Password recovery uses the same queue, but obsolete requests cannot renew indefinitely.
    assert_eq!(
        request(
            &failed,
            "POST",
            "/api/account/password/forgot",
            None,
            Some(json!({"email":address}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let reset = job(&pool, address, "password_reset").await;
    let reset_path = format!("/api/admin/notifications/{reset}/send");
    let reset_result = body(
        request(
            &healthy,
            "POST",
            &reset_path,
            Some(&owner),
            Some(reason.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(reset_result["status"], "sent");
    assert!(latest(&healthy, address, "password_reset").await["action_url"].is_string());
    sqlx::query("UPDATE customer_account_tokens SET created_at=now()-interval '2 hours',expires_at=now()-interval '1 hour' WHERE id=$1").bind(reset).execute(&pool).await.unwrap();
    sqlx::query("UPDATE notification_jobs SET status='pending',sent_at=NULL WHERE id=$1")
        .bind(reset)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        body(
            request(
                &healthy,
                "POST",
                &reset_path,
                Some(&owner),
                Some(reason.clone())
            )
            .await
        )
        .await["status"],
        "suppressed"
    );

    let login = request(
        &healthy,
        "POST",
        "/api/account/login",
        None,
        Some(json!({"email":address,"password":"account-notification-passphrase"})),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let customer_cookie = login
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find_map(|value| {
            let value = value.to_str().ok()?;
            value
                .starts_with("knitnprint_customer=")
                .then(|| value.split(';').next().unwrap().to_owned())
        })
        .unwrap();
    assert_eq!(
        request(
            &healthy,
            "POST",
            "/api/account/reauthenticate",
            Some(&customer_cookie),
            Some(json!({"password":"account-notification-passphrase"}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let target = "updated@example.test";
    assert_eq!(
        request(
            &failed,
            "POST",
            "/api/account/email/change",
            Some(&customer_cookie),
            Some(json!({"email":target}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    let change = job(&pool, target, "email_change").await;
    let change_path = format!("/api/admin/notifications/{change}/send");
    assert_eq!(
        body(
            request(
                &healthy,
                "POST",
                &change_path,
                Some(&owner),
                Some(reason.clone())
            )
            .await
        )
        .await["status"],
        "sent"
    );
    let change_secret = latest(&healthy, target, "email_change").await["action_url"]
        .as_str()
        .unwrap()
        .split("confirm_email=")
        .nth(1)
        .unwrap()
        .to_owned();
    assert_eq!(
        request(
            &healthy,
            "POST",
            "/api/account/email/confirm",
            None,
            Some(json!({"token":change_secret}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );

    // Removing an account cannot retain addresses in non-commercial email jobs.
    sqlx::query("DELETE FROM customer_accounts WHERE customer_id=$1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .unwrap();
    let retained: Vec<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT recipient_email,account_token_id FROM notification_jobs WHERE id IN ($1,$2,$3)",
    )
    .bind(id)
    .bind(reset)
    .bind(change)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(retained.len(), 3);
    assert!(
        retained
            .iter()
            .all(|(email, token)| email == "anonymized@knitnprint.invalid" && token.is_none())
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM notification_attempts WHERE notification_job_id=$1"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        4
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
