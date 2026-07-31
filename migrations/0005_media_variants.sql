CREATE TABLE media_variants (
    media_asset_id uuid NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    kind text NOT NULL,
    object_key text NOT NULL UNIQUE,
    content_type text NOT NULL DEFAULT 'image/webp',
    byte_size bigint NOT NULL,
    width integer NOT NULL,
    height integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (media_asset_id, kind),
    CONSTRAINT media_variants_kind_valid CHECK (kind IN ('thumbnail', 'card', 'detail')),
    CONSTRAINT media_variants_size_positive CHECK (byte_size > 0),
    CONSTRAINT media_variants_dimensions_positive CHECK (width > 0 AND height > 0)
);
