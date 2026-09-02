UPDATE products
SET shipping_profile_configured = true
WHERE shipping_profile_configured = false
  AND shipping_weight_grams = 500
  AND shipping_width_cm = 35
  AND shipping_length_cm = 50
  AND shipping_height_cm = 25
  AND shipping_units_per_package = 1;

COMMENT ON COLUMN products.shipping_profile_configured IS
'True after an administrator confirms the packed dimensions, including the legacy 35 x 50 x 25 cm and 500 g profile supplied for existing products.';
