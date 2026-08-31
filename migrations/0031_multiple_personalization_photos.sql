ALTER TABLE cart_lines
    ADD COLUMN customization_media_asset_ids uuid[] NOT NULL DEFAULT '{}'::uuid[];

ALTER TABLE order_lines
    ADD COLUMN customization_media_asset_ids uuid[] NOT NULL DEFAULT '{}'::uuid[];

UPDATE cart_lines
SET customization_media_asset_ids = ARRAY[customization_media_asset_id]
WHERE customization_media_asset_id IS NOT NULL;

UPDATE order_lines
SET customization_media_asset_ids = ARRAY[customization_media_asset_id]
WHERE customization_media_asset_id IS NOT NULL;

COMMENT ON COLUMN cart_lines.customization_media_asset_ids IS
    'All customer-uploaded photographs referenced by the area-based customization.';
COMMENT ON COLUMN order_lines.customization_media_asset_ids IS
    'Immutable list of customer-uploaded photographs referenced by the order customization.';
