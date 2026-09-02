ALTER TABLE products
ADD COLUMN shipping_weight_grams integer NOT NULL DEFAULT 500,
ADD COLUMN shipping_width_cm integer NOT NULL DEFAULT 35,
ADD COLUMN shipping_length_cm integer NOT NULL DEFAULT 50,
ADD COLUMN shipping_height_cm integer NOT NULL DEFAULT 25,
ADD COLUMN shipping_units_per_package integer NOT NULL DEFAULT 1,
ADD COLUMN shipping_profile_configured boolean NOT NULL DEFAULT false,
ADD CONSTRAINT products_shipping_weight_valid CHECK (shipping_weight_grams BETWEEN 1 AND 1000000),
ADD CONSTRAINT products_shipping_width_valid CHECK (shipping_width_cm BETWEEN 1 AND 300),
ADD CONSTRAINT products_shipping_length_valid CHECK (shipping_length_cm BETWEEN 1 AND 300),
ADD CONSTRAINT products_shipping_height_valid CHECK (shipping_height_cm BETWEEN 1 AND 300),
ADD CONSTRAINT products_shipping_capacity_valid CHECK (shipping_units_per_package BETWEEN 1 AND 100);

COMMENT ON COLUMN products.shipping_weight_grams IS
'Packed weight of one product unit used to calculate carrier quotes.';
COMMENT ON COLUMN products.shipping_width_cm IS
'Width of one parcel containing up to shipping_units_per_package units.';
COMMENT ON COLUMN products.shipping_length_cm IS
'Length of one parcel containing up to shipping_units_per_package units.';
COMMENT ON COLUMN products.shipping_height_cm IS
'Height of one parcel containing up to shipping_units_per_package units.';
COMMENT ON COLUMN products.shipping_units_per_package IS
'Maximum units of this product packed together in one parcel.';
COMMENT ON COLUMN products.shipping_profile_configured IS
'True only after an administrator confirms the packed weight and dimensions.';
