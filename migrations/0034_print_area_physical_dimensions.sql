UPDATE product_personalization
SET print_areas = (
    SELECT jsonb_agg(
        area || jsonb_build_object(
            'physical_width_cm', COALESCE(area -> 'physical_width_cm', '20'::jsonb),
            'physical_height_cm', COALESCE(area -> 'physical_height_cm', '20'::jsonb)
        )
        ORDER BY position
    )
    FROM jsonb_array_elements(print_areas) WITH ORDINALITY AS areas(area, position)
);

UPDATE product_personalization
SET views = (
    SELECT jsonb_agg(
        view_data || jsonb_build_object(
            'print_areas', (
                SELECT jsonb_agg(
                    area || jsonb_build_object(
                        'physical_width_cm', COALESCE(area -> 'physical_width_cm', '20'::jsonb),
                        'physical_height_cm', COALESCE(area -> 'physical_height_cm', '20'::jsonb)
                    )
                    ORDER BY area_position
                )
                FROM jsonb_array_elements(view_data -> 'print_areas')
                    WITH ORDINALITY AS areas(area, area_position)
            )
        )
        ORDER BY view_position
    )
    FROM jsonb_array_elements(views) WITH ORDINALITY AS configured_views(view_data, view_position)
);

COMMENT ON COLUMN product_personalization.views IS
    'Named product views with independent print areas and their physical dimensions in centimetres.';
