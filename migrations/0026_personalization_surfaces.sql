ALTER TABLE product_personalization
    ADD COLUMN preview_media_asset_id uuid REFERENCES media_assets(id) ON DELETE SET NULL,
    ADD COLUMN text_area_x integer NOT NULL DEFAULT 2500,
    ADD COLUMN text_area_y integer NOT NULL DEFAULT 6500,
    ADD COLUMN text_area_width integer NOT NULL DEFAULT 5000,
    ADD COLUMN text_area_height integer NOT NULL DEFAULT 2000,
    ADD CONSTRAINT product_personalization_preview_owned
        FOREIGN KEY (product_id, preview_media_asset_id)
        REFERENCES product_media (product_id, media_asset_id)
        ON DELETE SET NULL (preview_media_asset_id),
    ADD CONSTRAINT product_personalization_text_area_valid CHECK (
        text_area_x BETWEEN 0 AND 10000 AND text_area_y BETWEEN 0 AND 10000
        AND text_area_width BETWEEN 100 AND 10000 AND text_area_height BETWEEN 100 AND 10000
        AND text_area_x + text_area_width <= 10000
        AND text_area_y + text_area_height <= 10000
    );

COMMENT ON COLUMN product_personalization.preview_media_asset_id IS
    'Product photograph selected by staff as the base for the personalization editor.';
