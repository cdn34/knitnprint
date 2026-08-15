ALTER TABLE media_assets
    ADD COLUMN scan_status text NOT NULL DEFAULT 'pending',
    ADD COLUMN scanned_at timestamptz,
    ADD COLUMN scan_detail text;

UPDATE media_assets
SET scan_status = 'clean',
    scanned_at = completed_at
WHERE status = 'ready';

ALTER TABLE media_assets
    ADD CONSTRAINT media_assets_scan_status_valid
        CHECK (scan_status IN ('pending', 'clean', 'infected')),
    ADD CONSTRAINT media_assets_ready_scan_clean
        CHECK (status <> 'ready' OR scan_status = 'clean');

CREATE INDEX media_assets_failed_cleanup
    ON media_assets (created_at, id)
    WHERE status = 'failed';

COMMENT ON COLUMN media_assets.scan_status IS
    'Fail-closed malware scan result. Only clean assets may become public-ready.';
