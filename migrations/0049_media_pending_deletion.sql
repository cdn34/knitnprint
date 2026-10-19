ALTER TABLE media_assets DROP CONSTRAINT media_assets_status_valid;
ALTER TABLE media_assets ADD CONSTRAINT media_assets_status_valid
    CHECK (status IN ('pending', 'ready', 'failed', 'pending_deletion'));

CREATE INDEX media_assets_pending_deletion_cleanup
    ON media_assets (created_at, id)
    WHERE status = 'pending_deletion';
