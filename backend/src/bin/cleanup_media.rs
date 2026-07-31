use std::{env, process::ExitCode};

use knitprint_api::media::MediaStorage;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::main]
async fn main() -> ExitCode {
    match cleanup().await {
        Ok((removed, failed)) => {
            println!("removed {removed} abandoned media uploads; {failed} require retry");
            if failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("failed to clean abandoned media: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn cleanup() -> Result<(u64, u64), String> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let max_age_hours = parse_max_age(env::var("MEDIA_PENDING_MAX_HOURS").ok())?;
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .map_err(|error| format!("database connection failed: {error}"))?;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .map_err(|error| format!("database migration failed: {error}"))?;
    let storage = MediaStorage::from_env(false)
        .await?
        .ok_or("media storage is required")?;

    let claimed = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        WITH candidates AS (
            SELECT id
            FROM media_assets
            WHERE (status = 'pending' AND created_at < now() - make_interval(hours => $1))
               OR status = 'failed'
            ORDER BY created_at, id
            LIMIT 100
            FOR UPDATE SKIP LOCKED
        )
        UPDATE media_assets AS media
        SET status = 'failed'
        FROM candidates
        WHERE media.id = candidates.id
        RETURNING media.id, media.object_key
        "#,
    )
    .bind(max_age_hours)
    .fetch_all(&pool)
    .await
    .map_err(|error| format!("could not claim stale media: {error}"))?;

    let mut removed = 0;
    let mut failed = 0;
    for (id, original_key) in claimed {
        let keys = [
            original_key,
            variant_key(id, "thumbnail"),
            variant_key(id, "card"),
            variant_key(id, "detail"),
        ];
        let mut storage_failed = false;
        for key in keys {
            if storage
                .client
                .delete_object()
                .bucket(&storage.bucket)
                .key(key)
                .send()
                .await
                .is_err()
            {
                storage_failed = true;
            }
        }
        if storage_failed {
            failed += 1;
            continue;
        }
        let mut transaction = pool
            .begin()
            .await
            .map_err(|error| format!("cleanup transaction failed: {error}"))?;
        sqlx::query(
            r#"
            INSERT INTO audit_log (action, entity_type, entity_id, reason)
            VALUES ('media.abandoned_cleanup', 'media_asset', $1, $2)
            "#,
        )
        .bind(id.to_string())
        .bind(format!(
            "Upload remained incomplete for more than {max_age_hours} hours"
        ))
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("cleanup audit failed: {error}"))?;
        sqlx::query("DELETE FROM media_assets WHERE id = $1 AND status = 'failed'")
            .bind(id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| format!("media record cleanup failed: {error}"))?;
        transaction
            .commit()
            .await
            .map_err(|error| format!("cleanup commit failed: {error}"))?;
        removed += 1;
    }
    Ok((removed, failed))
}

fn parse_max_age(value: Option<String>) -> Result<i32, String> {
    match value {
        Some(value) => match value.parse::<i32>() {
            Ok(hours @ 1..=168) => Ok(hours),
            _ => Err("MEDIA_PENDING_MAX_HOURS must be between 1 and 168".into()),
        },
        None => Ok(24),
    }
}

fn variant_key(media_id: Uuid, kind: &str) -> String {
    format!("media-public/{media_id}/{kind}.webp")
}

#[cfg(test)]
mod tests {
    use super::parse_max_age;

    #[test]
    fn cleanup_age_defaults_and_stays_bounded() {
        assert_eq!(parse_max_age(None), Ok(24));
        assert_eq!(parse_max_age(Some("1".into())), Ok(1));
        assert!(parse_max_age(Some("0".into())).is_err());
        assert!(parse_max_age(Some("169".into())).is_err());
    }
}
