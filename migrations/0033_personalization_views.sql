ALTER TABLE product_personalization
    ADD COLUMN views jsonb NOT NULL DEFAULT
    '[{"id":"view-front","label":"Frente","media_id":null,"print_areas":[{"id":"area-1","label":"Área 1","x":2500,"y":2500,"width":5000,"height":5000}]}]'::jsonb;

UPDATE product_personalization
SET views = jsonb_build_array(jsonb_build_object(
    'id', 'view-front',
    'label', 'Frente',
    'media_id', CASE
        WHEN preview_media_asset_id IS NULL THEN 'null'::jsonb
        ELSE to_jsonb(preview_media_asset_id::text)
    END,
    'print_areas', print_areas
));

ALTER TABLE product_personalization
    ADD CONSTRAINT product_personalization_views_valid CHECK (
        jsonb_typeof(views) = 'array'
        AND jsonb_array_length(views) BETWEEN 1 AND 6
    );

COMMENT ON COLUMN product_personalization.views IS
    'Named product views, each with its product photograph and independent print areas.';
