CREATE TABLE shipping_package_profiles (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    width_cm integer NOT NULL,
    length_cm integer NOT NULL,
    height_cm integer NOT NULL,
    empty_weight_grams integer NOT NULL DEFAULT 0,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT shipping_package_profiles_name_not_blank CHECK (btrim(name) <> ''),
    CONSTRAINT shipping_package_profiles_name_length CHECK (char_length(name) <= 120),
    CONSTRAINT shipping_package_profiles_width_valid CHECK (width_cm BETWEEN 1 AND 300),
    CONSTRAINT shipping_package_profiles_length_valid CHECK (length_cm BETWEEN 1 AND 300),
    CONSTRAINT shipping_package_profiles_height_valid CHECK (height_cm BETWEEN 1 AND 300),
    CONSTRAINT shipping_package_profiles_empty_weight_valid CHECK (empty_weight_grams BETWEEN 0 AND 100000)
);

CREATE UNIQUE INDEX shipping_package_profiles_name_unique
ON shipping_package_profiles (lower(name));

ALTER TABLE products
ADD COLUMN shipping_package_profile_id uuid
REFERENCES shipping_package_profiles(id) ON DELETE RESTRICT;

WITH legacy_dimensions AS (
    SELECT DISTINCT
        shipping_width_cm AS width_cm,
        shipping_length_cm AS length_cm,
        shipping_height_cm AS height_cm
    FROM products
    WHERE shipping_profile_configured = true
), inserted AS (
    INSERT INTO shipping_package_profiles (
        id, name, width_cm, length_cm, height_cm, empty_weight_grams
    )
    SELECT
        gen_random_uuid(),
        'Embalagem existente ' || row_number() OVER (
            ORDER BY width_cm, length_cm, height_cm
        ),
        width_cm,
        length_cm,
        height_cm,
        0
    FROM legacy_dimensions
    RETURNING id, width_cm, length_cm, height_cm
)
UPDATE products product
SET shipping_package_profile_id = profile.id
FROM inserted profile
WHERE product.shipping_profile_configured = true
  AND product.shipping_width_cm = profile.width_cm
  AND product.shipping_length_cm = profile.length_cm
  AND product.shipping_height_cm = profile.height_cm;

CREATE INDEX products_shipping_package_profile_idx
ON products (shipping_package_profile_id)
WHERE shipping_package_profile_id IS NOT NULL;

COMMENT ON TABLE shipping_package_profiles IS
'Reusable parcel definitions selected by products and used to compose Packlink shipments.';
COMMENT ON COLUMN shipping_package_profiles.empty_weight_grams IS
'Weight of the empty packaging, added once to every composed parcel.';
COMMENT ON COLUMN products.shipping_package_profile_id IS
'Reusable parcel profile selected for this product. Unit weight and package capacity remain product-specific.';
