-- Text and combined products previously used a second text-only surface. Preserve
-- that staff-selected surface as the new shared maximum print area.
UPDATE product_personalization
SET area_x = text_area_x,
    area_y = text_area_y,
    area_width = text_area_width,
    area_height = text_area_height
WHERE mode IN ('text', 'photo_text');

-- Keep the legacy columns synchronized for clients deployed before this change.
UPDATE product_personalization
SET text_area_x = area_x,
    text_area_y = area_y,
    text_area_width = area_width,
    text_area_height = area_height;

COMMENT ON COLUMN product_personalization.area_x IS
    'Horizontal position of the shared maximum print area in basis points relative to the selected product image.';
COMMENT ON COLUMN product_personalization.text_area_x IS
    'Legacy compatibility field synchronized with area_x.';
