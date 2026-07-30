CREATE TABLE categories (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    slug text NOT NULL UNIQUE,
    description text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT categories_name_not_blank CHECK (length(btrim(name)) > 0),
    CONSTRAINT categories_slug_valid CHECK (slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$')
);

CREATE TABLE products (
    id uuid PRIMARY KEY,
    title text NOT NULL,
    slug text NOT NULL UNIQUE,
    description text NOT NULL DEFAULT '',
    status text NOT NULL DEFAULT 'draft',
    search_keywords text NOT NULL DEFAULT '',
    search_document tsvector GENERATED ALWAYS AS (
        to_tsvector('simple', title || ' ' || description || ' ' || search_keywords)
    ) STORED,
    published_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT products_title_not_blank CHECK (length(btrim(title)) > 0),
    CONSTRAINT products_slug_valid CHECK (slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'),
    CONSTRAINT products_status_valid CHECK (status IN ('draft', 'active', 'archived')),
    CONSTRAINT products_publish_state_valid CHECK (
        (status = 'active' AND published_at IS NOT NULL) OR
        (status <> 'active')
    )
);

CREATE INDEX products_public_order ON products (published_at DESC, id)
    WHERE status = 'active';
CREATE INDEX products_search ON products USING gin (search_document);

CREATE TABLE product_variants (
    id uuid PRIMARY KEY,
    product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    title text NOT NULL,
    sku text NOT NULL UNIQUE,
    price_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    option_values jsonb NOT NULL DEFAULT '{}'::jsonb,
    position integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT product_variants_title_not_blank CHECK (length(btrim(title)) > 0),
    CONSTRAINT product_variants_sku_not_blank CHECK (length(btrim(sku)) > 0),
    CONSTRAINT product_variants_price_non_negative CHECK (price_minor >= 0),
    CONSTRAINT product_variants_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT product_variants_position_non_negative CHECK (position >= 0)
);

CREATE INDEX product_variants_product_order
    ON product_variants (product_id, position, id);

CREATE TABLE product_categories (
    product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    category_id uuid NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    position integer NOT NULL DEFAULT 0,
    PRIMARY KEY (product_id, category_id)
);

CREATE TABLE media_assets (
    id uuid PRIMARY KEY,
    object_key text NOT NULL UNIQUE,
    content_type text NOT NULL,
    byte_size bigint NOT NULL,
    width integer,
    height integer,
    status text NOT NULL DEFAULT 'pending',
    created_by_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    CONSTRAINT media_assets_object_key_not_blank CHECK (length(btrim(object_key)) > 0),
    CONSTRAINT media_assets_byte_size_positive CHECK (byte_size > 0),
    CONSTRAINT media_assets_dimensions_positive CHECK (
        (width IS NULL OR width > 0) AND (height IS NULL OR height > 0)
    ),
    CONSTRAINT media_assets_status_valid CHECK (status IN ('pending', 'ready', 'failed'))
);

CREATE TABLE product_media (
    product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    media_asset_id uuid NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    alt_text text NOT NULL DEFAULT '',
    position integer NOT NULL DEFAULT 0,
    PRIMARY KEY (product_id, media_asset_id),
    CONSTRAINT product_media_position_non_negative CHECK (position >= 0)
);

CREATE TABLE variant_media (
    variant_id uuid NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    media_asset_id uuid NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    alt_text text NOT NULL DEFAULT '',
    position integer NOT NULL DEFAULT 0,
    PRIMARY KEY (variant_id, media_asset_id),
    CONSTRAINT variant_media_position_non_negative CHECK (position >= 0)
);
