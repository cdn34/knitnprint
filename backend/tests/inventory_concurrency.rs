use std::{env, str::FromStr};

use knitprint_api::inventory::{
    InventoryOperationError, commit, get_availability, release, reserve,
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use uuid::Uuid;

#[tokio::test]
async fn reservation_release_and_commit_preserve_inventory_invariants() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let (admin, pool, schema) = isolated_database(&database_url).await;
    let variant_id = inventory_fixture(&pool, 10).await;

    let initial = get_availability(&pool, variant_id)
        .await
        .expect("availability lookup should succeed")
        .expect("inventory should exist");
    assert_eq!(
        (
            initial.available_quantity,
            initial.reserved_quantity,
            initial.committed_quantity,
        ),
        (10, 0, 0)
    );

    let reserved = reserve(&pool, variant_id, 4, "Checkout reservation")
        .await
        .expect("stock should reserve");
    assert_eq!(
        (
            reserved.available_quantity,
            reserved.reserved_quantity,
            reserved.committed_quantity,
        ),
        (6, 4, 0)
    );

    let released = release(&pool, variant_id, 1, "Cart quantity reduced")
        .await
        .expect("reserved stock should release");
    assert_eq!(
        (
            released.available_quantity,
            released.reserved_quantity,
            released.committed_quantity,
        ),
        (7, 3, 0)
    );

    let committed = commit(&pool, variant_id, 3, "Order payment captured")
        .await
        .expect("reserved stock should commit");
    assert_eq!(
        (
            committed.available_quantity,
            committed.reserved_quantity,
            committed.committed_quantity,
        ),
        (7, 0, 3)
    );

    assert!(matches!(
        release(&pool, variant_id, 1, "Invalid extra release").await,
        Err(InventoryOperationError::InsufficientReserved)
    ));
    let backordered = reserve(&pool, variant_id, 8, "Backorder reservation")
        .await
        .expect("demand beyond current stock should be recorded");
    assert_eq!(
        (
            backordered.available_quantity,
            backordered.reserved_quantity,
            backordered.committed_quantity,
        ),
        (-1, 8, 3)
    );
    assert!(matches!(
        reserve(&pool, variant_id, 0, "Invalid quantity").await,
        Err(InventoryOperationError::InvalidQuantity)
    ));
    assert!(matches!(
        reserve(&pool, variant_id, 1, " ").await,
        Err(InventoryOperationError::InvalidReason)
    ));
    assert!(matches!(
        reserve(&pool, Uuid::now_v7(), 1, "Missing inventory").await,
        Err(InventoryOperationError::NotFound)
    ));

    let movements: Vec<(String, i64, i64, String, Option<Uuid>)> = sqlx::query_as(
        r#"
        SELECT movement_type, quantity_delta, resulting_available_quantity, reason,
               actor_staff_user_id
        FROM inventory_movements
        WHERE variant_id = $1
        ORDER BY created_at, id
        "#,
    )
    .bind(variant_id)
    .fetch_all(&pool)
    .await
    .expect("movement history should be readable");
    assert_eq!(
        movements,
        vec![
            (
                "reservation".into(),
                -4,
                6,
                "Checkout reservation".into(),
                None,
            ),
            ("release".into(), 1, 7, "Cart quantity reduced".into(), None,),
            (
                "commitment".into(),
                -3,
                7,
                "Order payment captured".into(),
                None,
            ),
            (
                "reservation".into(),
                -8,
                -1,
                "Backorder reservation".into(),
                None,
            ),
        ]
    );

    cleanup(admin, pool, &schema).await;
}

#[tokio::test]
async fn concurrent_reservations_record_demand_beyond_available_stock() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let (admin, pool, schema) = isolated_database(&database_url).await;
    let variant_id = inventory_fixture(&pool, 5).await;

    let mut reservations = Vec::new();
    for _ in 0..12 {
        let pool = pool.clone();
        reservations.push(tokio::spawn(async move {
            reserve(&pool, variant_id, 1, "Concurrent checkout reservation").await
        }));
    }

    let mut succeeded = 0;
    for reservation in reservations {
        match reservation.await.expect("reservation task should complete") {
            Ok(_) => succeeded += 1,
            Err(error) => panic!("unexpected reservation error: {error}"),
        }
    }
    assert_eq!(succeeded, 12);

    let final_state = get_availability(&pool, variant_id)
        .await
        .expect("availability lookup should succeed")
        .expect("inventory should exist");
    assert_eq!(final_state.available_quantity, -7);
    assert_eq!(final_state.reserved_quantity, 12);
    assert_eq!(final_state.committed_quantity, 0);
    assert!(final_state.low_stock);

    let movement_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM inventory_movements WHERE variant_id = $1")
            .bind(variant_id)
            .fetch_one(&pool)
            .await
            .expect("movement count should be readable");
    assert_eq!(movement_count, 12);

    cleanup(admin, pool, &schema).await;
}

async fn isolated_database(database_url: &str) -> (PgPool, PgPool, String) {
    let schema = format!("inventory_test_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .expect("test database should be available");
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&admin)
        .await
        .expect("test schema should be created");

    let options = PgConnectOptions::from_str(database_url).expect("DATABASE_URL should be valid");
    let search_path = format!(r#"SET search_path TO "{schema}", public"#);
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query(&search_path).execute(connection).await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .expect("isolated test pool should connect");
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations should run in the isolated schema");

    (admin, pool, schema)
}

async fn inventory_fixture(pool: &PgPool, available_quantity: i64) -> Uuid {
    let product_id = Uuid::now_v7();
    let variant_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO products (id, title, slug, description, search_keywords)
        VALUES ($1, 'Concurrency test product', $2, '', '')
        "#,
    )
    .bind(product_id)
    .bind(format!("inventory-test-{}", product_id.simple()))
    .execute(pool)
    .await
    .expect("product fixture should be inserted");
    sqlx::query(
        r#"
        INSERT INTO product_variants (
            id, product_id, title, sku, price_minor, currency, option_values
        )
        VALUES ($1, $2, 'Default', $3, 1000, 'EUR', '{}'::jsonb)
        "#,
    )
    .bind(variant_id)
    .bind(product_id)
    .bind(format!("INVENTORY-{}", variant_id.simple()))
    .execute(pool)
    .await
    .expect("variant fixture should be inserted");
    sqlx::query("UPDATE inventory_items SET available_quantity = $2 WHERE variant_id = $1")
        .bind(variant_id)
        .bind(available_quantity)
        .execute(pool)
        .await
        .expect("inventory fixture should be stocked");
    variant_id
}

async fn cleanup(admin: PgPool, pool: PgPool, schema: &str) {
    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}
