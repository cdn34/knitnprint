ALTER TABLE categories
    ADD COLUMN position integer;

WITH ordered_categories AS (
    SELECT id, (row_number() OVER (ORDER BY name, id) - 1)::integer AS position
    FROM categories
)
UPDATE categories category
SET position = ordered_categories.position
FROM ordered_categories
WHERE ordered_categories.id = category.id;

ALTER TABLE categories
    ALTER COLUMN position SET DEFAULT 0,
    ALTER COLUMN position SET NOT NULL,
    ADD CONSTRAINT categories_position_non_negative CHECK (position >= 0);

CREATE INDEX categories_storefront_order ON categories (position, id);
