ALTER TABLE product_personalization
    ALTER COLUMN text_area_y SET DEFAULT 3000,
    ALTER COLUMN text_area_height SET DEFAULT 2500;

UPDATE product_personalization
SET text_area_y = 3000,
    text_area_height = 2500
WHERE text_area_x = 2500
  AND text_area_y = 6500
  AND text_area_width = 5000
  AND text_area_height = 2000;

