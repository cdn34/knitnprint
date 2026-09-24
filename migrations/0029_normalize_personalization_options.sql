UPDATE product_personalization AS personalization
SET allowed_fonts = '["Roboto", "Montserrat", "Playfair Display", "Dancing Script", "Pacifico"]'::jsonb
WHERE CASE
    WHEN jsonb_typeof(personalization.allowed_fonts) = 'array' THEN
        jsonb_array_length(personalization.allowed_fonts) = 0
        OR EXISTS (
            SELECT 1
            FROM jsonb_array_elements_text(personalization.allowed_fonts) AS font(value)
            WHERE font.value NOT IN ('Roboto', 'Montserrat', 'Playfair Display', 'Dancing Script', 'Pacifico')
        )
    ELSE true
END;

UPDATE product_personalization AS personalization
SET allowed_colors = '["#111111", "#ffffff", "#9c5263", "#1f4f78", "#b3232f"]'::jsonb
WHERE CASE
    WHEN jsonb_typeof(personalization.allowed_colors) = 'array' THEN
        jsonb_array_length(personalization.allowed_colors) = 0
        OR EXISTS (
            SELECT 1
            FROM jsonb_array_elements_text(personalization.allowed_colors) AS color(value)
            WHERE color.value !~ '^#[0-9A-Fa-f]{6}$'
        )
    ELSE true
END;
