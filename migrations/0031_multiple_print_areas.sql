ALTER TABLE product_personalization
    ADD COLUMN print_areas jsonb NOT NULL DEFAULT
    '[{"id":"area-1","label":"Área 1","x":2500,"y":2500,"width":5000,"height":5000}]'::jsonb;

UPDATE product_personalization
SET print_areas = jsonb_build_array(jsonb_build_object(
    'id', 'area-1',
    'label', 'Área 1',
    'x', area_x,
    'y', area_y,
    'width', area_width,
    'height', area_height
));

ALTER TABLE product_personalization
    ADD CONSTRAINT product_personalization_print_areas_valid CHECK (
        jsonb_typeof(print_areas) = 'array'
        AND jsonb_array_length(print_areas) BETWEEN 1 AND 8
    );

COMMENT ON COLUMN product_personalization.print_areas IS
    'Named print surfaces with basis-point coordinates relative to the selected product image.';
