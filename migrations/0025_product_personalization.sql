CREATE TABLE product_personalization (
    product_id uuid PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    mode text NOT NULL DEFAULT 'none',
    area_x integer NOT NULL DEFAULT 2500,
    area_y integer NOT NULL DEFAULT 2500,
    area_width integer NOT NULL DEFAULT 5000,
    area_height integer NOT NULL DEFAULT 5000,
    text_max_characters integer NOT NULL DEFAULT 35,
    text_min_size integer NOT NULL DEFAULT 12,
    text_max_size integer NOT NULL DEFAULT 72,
    allowed_fonts jsonb NOT NULL DEFAULT '["Arial"]'::jsonb,
    allowed_colors jsonb NOT NULL DEFAULT '["#111111"]'::jsonb,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT product_personalization_mode_valid CHECK (mode IN ('none', 'photo', 'text', 'photo_text')),
    CONSTRAINT product_personalization_area_valid CHECK (
        area_x BETWEEN 0 AND 10000 AND area_y BETWEEN 0 AND 10000
        AND area_width BETWEEN 100 AND 10000 AND area_height BETWEEN 100 AND 10000
        AND area_x + area_width <= 10000 AND area_y + area_height <= 10000
    ),
    CONSTRAINT product_personalization_text_valid CHECK (
        text_max_characters BETWEEN 1 AND 500
        AND text_min_size BETWEEN 8 AND 200
        AND text_max_size BETWEEN text_min_size AND 300
        AND jsonb_typeof(allowed_fonts) = 'array'
        AND jsonb_array_length(allowed_fonts) BETWEEN 1 AND 20
        AND jsonb_typeof(allowed_colors) = 'array'
        AND jsonb_array_length(allowed_colors) BETWEEN 1 AND 30
    )
);

ALTER TABLE cart_lines
    DROP CONSTRAINT cart_lines_cart_id_variant_id_key,
    ADD COLUMN customization jsonb,
    ADD COLUMN customization_media_asset_id uuid REFERENCES media_assets(id) ON DELETE RESTRICT;

CREATE UNIQUE INDEX cart_lines_standard_variant_unique
    ON cart_lines (cart_id, variant_id)
    WHERE customization IS NULL;

ALTER TABLE order_lines
    ADD COLUMN customization jsonb,
    ADD COLUMN customization_media_asset_id uuid REFERENCES media_assets(id) ON DELETE RESTRICT;

COMMENT ON COLUMN product_personalization.area_x IS 'Horizontal position in basis points relative to the primary product image.';
COMMENT ON COLUMN product_personalization.area_y IS 'Vertical position in basis points relative to the primary product image.';
COMMENT ON COLUMN cart_lines.customization IS 'Customer composition using normalized coordinates and administrator-approved options.';
COMMENT ON COLUMN order_lines.customization IS 'Immutable snapshot of the customer personalization composition.';
