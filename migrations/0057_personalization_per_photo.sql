-- Keep the existing areas and measurements, and give each photograph its own permissions.
UPDATE product_personalization
SET views = (
    SELECT jsonb_agg(
        CASE WHEN item ? 'mode' THEN item
             ELSE item || jsonb_build_object('mode', product_personalization.mode) END
        ORDER BY ordinality
    )
    FROM jsonb_array_elements(views) WITH ORDINALITY AS entries(item, ordinality)
);

ALTER TABLE product_personalization
    DROP CONSTRAINT product_personalization_print_areas_valid,
    ADD CONSTRAINT product_personalization_print_areas_valid CHECK (
        jsonb_typeof(print_areas) = 'array'
        AND jsonb_array_length(print_areas) BETWEEN 0 AND 8
    ),
    DROP CONSTRAINT product_personalization_views_valid,
    ADD CONSTRAINT product_personalization_views_valid CHECK (
        jsonb_typeof(views) = 'array'
        AND jsonb_array_length(views) BETWEEN 1 AND 100
    );

COMMENT ON COLUMN product_personalization.views IS
    'Product photographs with independent personalization mode, zero to eight print areas, and production measurements.';
