use std::{env, io::Cursor, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use knitnprint_api::{AppState, app, auth::hash_password, b2b, email::EmailService};
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn business_request_attachment_permissions_and_durable_notification() {
    let Ok(url) = env::var("DATABASE_URL") else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("b2b_test_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\" "))
        .execute(&admin)
        .await
        .unwrap();
    let search_path = format!("{schema},public");
    let options = PgConnectOptions::from_str(&url)
        .unwrap()
        .options([("search_path", search_path.as_str())]);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let email = EmailService::development("http://localhost:3000");
    let router = app(AppState {
        database: Some(pool.clone()),
        email: email.clone(),
        ..AppState::default()
    });
    let mut image = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(24, 12)
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    let image = image.into_inner();
    let mut input = json!({
        "request_key": Uuid::new_v4(), "company": "Business <test>", "contact_name": "Test contact",
        "email": "business@example.invalid", "phone": "000000000", "product_type": "clothing",
        "quantity": 25, "notes": "Two colours.\nDelivery next month.",
        "attachment_name": "brand.png", "attachment_base64": STANDARD.encode(&image),
    });
    let response = request(
        &router,
        "POST",
        "/api/b2b/requests",
        None,
        Some(input.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let receipt = json_response(response).await;
    let id = receipt["id"].as_str().unwrap();
    assert_eq!(receipt["reference"], "B2B-1");
    let path = format!("/api/admin/b2b/requests/{id}");
    let file_path = format!("{path}/attachment");

    // A lost response can be retried without a second request or notification.
    let repeated = request(
        &router,
        "POST",
        "/api/b2b/requests",
        None,
        Some(input.clone()),
    )
    .await;
    assert_eq!(repeated.status(), StatusCode::OK);
    assert_eq!(json_response(repeated).await, receipt);
    input["quantity"] = json!(26);
    assert_eq!(
        request(
            &router,
            "POST",
            "/api/b2b/requests",
            None,
            Some(input.clone())
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b2b_requests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b2b_notification_jobs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );

    // Originals survive independently of browser state or object storage.
    let stored: Vec<u8> = sqlx::query_scalar("SELECT attachment_bytes FROM b2b_requests")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, image);
    for route in ["/api/admin/b2b/requests", path.as_str(), file_path.as_str()] {
        assert_eq!(
            request(&router, "GET", route, None, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    for (address, role) in [("owner@b2b.test", "owner"), ("limited@b2b.test", "staff")] {
        sqlx::query("INSERT INTO staff_users (id,email,display_name,password_hash,role) VALUES ($1,$2,'B2B test',$3,$4)")
            .bind(Uuid::now_v7()).bind(address).bind(hash_password("integration-test-passphrase").unwrap()).bind(role)
            .execute(&pool).await.unwrap();
    }
    let limited = login(&router, "limited@b2b.test").await;
    for route in ["/api/admin/b2b/requests", path.as_str(), file_path.as_str()] {
        assert_eq!(
            request(&router, "GET", route, Some(&limited), None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        request(&router, "DELETE", &path, None, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("INSERT INTO staff_capabilities (staff_user_id,capability_name) SELECT id,'orders.read' FROM staff_users WHERE email='limited@b2b.test'")
        .execute(&pool).await.unwrap();
    assert_eq!(
        request(&router, "GET", &path, Some(&limited), None)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(&router, "DELETE", &path, Some(&limited), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let owner = login(&router, "owner@b2b.test").await;
    let detail = request(&router, "GET", &path, Some(&owner), None).await;
    assert_eq!(detail.headers()[header::CACHE_CONTROL], "private, no-store");
    let detail = json_response(detail).await;
    assert_eq!(detail["company"], "Business <test>");
    assert_eq!(detail["has_preview"], true);
    assert_eq!(detail["attachment_size"], image.len());
    assert_eq!(detail["notification_status"], "pending");
    let original = request(&router, "GET", &file_path, Some(&owner), None).await;
    assert_eq!(
        original.headers()[header::CONTENT_DISPOSITION],
        "attachment; filename=\"brand.png\""
    );
    assert_eq!(
        to_bytes(original.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        image.as_slice()
    );
    let preview = request(
        &router,
        "GET",
        &format!("{file_path}?preview=true"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(preview.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(
        preview.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert!(
        image::load_from_memory(&to_bytes(preview.into_body(), usize::MAX).await.unwrap()).is_ok()
    );
    assert_eq!(
        request(
            &router,
            "PUT",
            &format!("{path}/status"),
            Some(&owner),
            Some(json!({"status":"in_progress"}))
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        json_response(request(&router, "GET", &path, Some(&owner), None).await).await["status"],
        "in_progress"
    );

    // Failure does not remove the request; a later retry sends the support notification.
    let failed = b2b::deliver_due(&pool, &EmailService::default(), 25)
        .await
        .unwrap();
    assert_eq!(failed.failed, 1);
    let status: (String, i32) =
        sqlx::query_as("SELECT status,attempt_count FROM b2b_notification_jobs")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, ("pending".into(), 1));
    sqlx::query("UPDATE b2b_notification_jobs SET next_attempt_at=now()")
        .execute(&pool)
        .await
        .unwrap();
    let sent = b2b::deliver_due(&pool, &email, 25).await.unwrap();
    assert_eq!(sent.sent, 1);
    assert_eq!(b2b::deliver_due(&pool, &email, 25).await.unwrap().sent, 0);
    let mailbox = json_response(
        request(
            &router,
            "GET",
            "/api/development/emails/latest?to=support%40knitnprint.com&kind=b2b_request",
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(mailbox["to"], "support@knitnprint.com");
    assert!(mailbox["subject"].as_str().unwrap().contains("B2B-1"));
    assert_eq!(
        json_response(request(&router, "GET", &path, Some(&owner), None).await).await["notification_status"],
        "sent"
    );

    input["request_key"] = json!(Uuid::new_v4());
    input["attachment_base64"] = json!(STANDARD.encode(b"<script>not a PNG</script>"));
    assert_eq!(
        request(&router, "POST", "/api/b2b/requests", None, Some(input))
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM b2b_requests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    // The route accepts attachments above the API's normal 1 MiB JSON limit.
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"12\"><metadata>{}</metadata><rect width=\"24\" height=\"12\"/></svg>",
        "x".repeat(1100 * 1024)
    );
    let large = json!({
        "request_key": Uuid::new_v4(), "company": "Vector test", "contact_name": "Test contact",
        "email": "vector@example.invalid", "phone": "000000000", "product_type": "other",
        "quantity": 10, "notes": "", "attachment_name": "logo.svg", "attachment_base64": STANDARD.encode(svg.as_bytes()),
    });
    let created = request(&router, "POST", "/api/b2b/requests", None, Some(large)).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let large_receipt = json_response(created).await;
    let restarted = app(AppState {
        database: Some(pool.clone()),
        email,
        ..AppState::default()
    });
    let svg_path = format!(
        "/api/admin/b2b/requests/{}/attachment?preview=true",
        large_receipt["id"].as_str().unwrap()
    );
    let svg_preview = request(&restarted, "GET", &svg_path, Some(&owner), None).await;
    assert_eq!(svg_preview.status(), StatusCode::OK);
    assert_eq!(svg_preview.headers()[header::CONTENT_TYPE], "image/svg+xml");
    assert!(
        svg_preview.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("sandbox")
    );
    assert_eq!(
        to_bytes(svg_preview.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        svg.as_bytes()
    );
    let large_id = large_receipt["id"].as_str().unwrap();
    let large_path = format!("/api/admin/b2b/requests/{large_id}");
    // Active delivery is protected; deletion cannot race the support notification.
    sqlx::query(
        "UPDATE b2b_notification_jobs SET status='processing',locked_at=now() WHERE request_id=$1",
    )
    .bind(Uuid::parse_str(large_id).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let busy = request(&restarted, "DELETE", &large_path, Some(&owner), None).await;
    assert_eq!(busy.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_response(busy).await["error"]["code"],
        "b2b_notification_in_progress"
    );
    assert_eq!(
        request(&restarted, "GET", &svg_path, Some(&owner), None)
            .await
            .status(),
        StatusCode::OK
    );
    sqlx::query(
        "UPDATE b2b_notification_jobs SET status='pending',locked_at=NULL WHERE request_id=$1",
    )
    .bind(Uuid::parse_str(large_id).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        request(&restarted, "DELETE", &large_path, Some(&owner), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    for route in [&large_path, &svg_path, &format!("{large_path}/attachment")] {
        assert_eq!(
            request(&restarted, "GET", route, Some(&owner), None)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM b2b_notification_jobs WHERE request_id=$1"
        )
        .bind(Uuid::parse_str(large_id).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_log WHERE action='b2b.delete' AND entity_id=$1"
        )
        .bind(large_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    let inbox = json_response(
        request(
            &restarted,
            "GET",
            "/api/admin/b2b/requests",
            Some(&owner),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(inbox.as_array().unwrap().len(), 1);
    assert_eq!(inbox[0]["id"], id);
    // Sent requests can also be removed; repeating the deletion returns 404.
    assert_eq!(
        request(&restarted, "DELETE", &path, Some(&owner), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&restarted, "DELETE", &path, Some(&owner), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    for table in ["b2b_requests", "b2b_notification_jobs"] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }
    assert_eq!(
        request(&restarted, "GET", &file_path, Some(&owner), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        b2b::deliver_due(
            &pool,
            &EmailService::development("http://localhost:3000"),
            25
        )
        .await
        .unwrap()
        .sent,
        0
    );
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}

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
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    router
        .clone()
        .oneshot(
            request
                .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json_response(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn login(router: &axum::Router, email: &str) -> String {
    let response = request(
        router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({"email":email,"password":"integration-test-passphrase"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}
