use std::collections::BTreeSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::object_storage::ObjectStorage;

/// Hide stale uploads before touching storage. Keep hidden assets and their keys
/// until every file has been deleted, so failures and process exits are retryable.
pub async fn cleanup_media(
    pool: &PgPool,
    storage: &ObjectStorage,
    max_age_hours: i32,
) -> Result<(u64, u64), sqlx::Error> {
    let claimed = sqlx::query_scalar::<_, Uuid>(
        r#"
        WITH candidates AS (
            SELECT id FROM media_assets
            WHERE (status = 'pending' AND created_at < now() - make_interval(hours => $1))
               OR status IN ('failed', 'pending_deletion')
               OR (
                    status = 'ready'
                    AND created_by_staff_user_id IS NULL
                    AND personalization_cart_id IS NULL
                    AND NOT EXISTS (
                        SELECT 1 FROM cart_lines AS line
                        WHERE media_assets.id = line.customization_media_asset_id
                           OR media_assets.id = ANY(line.customization_media_asset_ids)
                           OR EXISTS (SELECT 1 FROM jsonb_array_elements(line.personalization_previews) preview WHERE preview->>'media_id' = media_assets.id::text)
                    )
                    AND NOT EXISTS (
                        SELECT 1 FROM order_lines AS line
                        WHERE media_assets.id = line.customization_media_asset_id
                           OR media_assets.id = ANY(line.customization_media_asset_ids)
                           OR EXISTS (SELECT 1 FROM jsonb_array_elements(line.personalization_previews) preview WHERE preview->>'media_id' = media_assets.id::text)
                    )
               )
            ORDER BY created_at, id LIMIT 100 FOR UPDATE SKIP LOCKED
        )
        UPDATE media_assets AS media
        SET status = CASE WHEN media.status = 'pending_deletion' THEN 'pending_deletion' ELSE 'failed' END
        FROM candidates WHERE media.id = candidates.id
        RETURNING media.id
        "#,
    )
    .bind(max_age_hours)
    .fetch_all(pool)
    .await?;

    let mut removed = 0;
    let mut failed = 0;
    for id in claimed {
        // Hold the row lock through deletion to avoid overlapping workers.
        let mut tx = pool.begin().await?;
        let asset = sqlx::query_as::<_, (String, String)>(
            "SELECT object_key, status FROM media_assets WHERE id = $1 AND status IN ('failed', 'pending_deletion') FOR UPDATE SKIP LOCKED",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((original_key, status)) = asset else {
            continue;
        };
        let mut keys: BTreeSet<String> =
            sqlx::query_scalar("SELECT object_key FROM media_variants WHERE media_asset_id = $1")
                .bind(id)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .collect();
        keys.insert(original_key);
        // A failed upload may have written variants before recording them in SQL.
        for kind in ["thumbnail", "card", "detail"] {
            keys.insert(format!("media-public/{id}/{kind}.webp"));
        }
        let mut storage_failed = false;
        for key in keys {
            if let Err(error) = storage.delete(&key).await {
                tracing::warn!(%error, object_key = %key, "media cleanup deletion failed");
                storage_failed = true;
            }
        }
        if storage_failed {
            failed += 1;
            continue;
        }
        let (action, reason) = if status == "pending_deletion" {
            (
                "media.deleted_cleanup",
                "Photo was marked for deletion".to_owned(),
            )
        } else {
            (
                "media.abandoned_cleanup",
                format!(
                    "Upload was abandoned or remained incomplete for more than {max_age_hours} hours"
                ),
            )
        };
        sqlx::query(
            "INSERT INTO audit_log (action, entity_type, entity_id, reason) VALUES ($1, 'media_asset', $2, $3)",
        )
        .bind(action)
        .bind(id.to_string())
        .bind(reason)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM media_assets WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        removed += 1;
    }
    Ok((removed, failed))
}
