-- Retire the previous delivery-pricing model. Existing order snapshots and
-- delivery addresses remain independent of these mutable configuration tables.
ALTER TABLE carts DROP COLUMN shipping_method_id, DROP COLUMN shipping_quote_id;
DROP TABLE cart_shipping_quotes;
DROP TABLE shipping_methods;
DROP TABLE shipping_zones;

ALTER TABLE products
    DROP COLUMN shipping_package_profile_id,
    DROP COLUMN shipping_weight_grams,
    DROP COLUMN shipping_width_cm,
    DROP COLUMN shipping_length_cm,
    DROP COLUMN shipping_height_cm,
    DROP COLUMN shipping_units_per_package,
    DROP COLUMN shipping_profile_configured;
DROP TABLE shipping_package_profiles;
