ALTER TABLE order_lines
    ADD COLUMN personalization_context jsonb;

COMMENT ON COLUMN order_lines.personalization_context IS
    'Immutable administrator-approved product image, print-area geometry and article calibration used to reconstruct the ordered personalization.';

CREATE TABLE order_line_personalization_media (
    order_line_id uuid NOT NULL REFERENCES order_lines(id) ON DELETE RESTRICT,
    media_asset_id uuid NOT NULL REFERENCES media_assets(id) ON DELETE RESTRICT,
    position integer NOT NULL,
    PRIMARY KEY (order_line_id, media_asset_id),
    UNIQUE (order_line_id, position),
    CONSTRAINT order_line_personalization_media_position_non_negative CHECK (position >= 0)
);

INSERT INTO order_line_personalization_media (order_line_id, media_asset_id, position)
SELECT line.id, media.id, media.ordinality::integer - 1
FROM order_lines line
CROSS JOIN LATERAL unnest(line.customization_media_asset_ids) WITH ORDINALITY AS media(id, ordinality);

CREATE INDEX order_line_personalization_media_asset
    ON order_line_personalization_media (media_asset_id, order_line_id);

CREATE TRIGGER order_line_personalization_media_immutable
BEFORE UPDATE OR DELETE ON order_line_personalization_media
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

COMMENT ON TABLE order_line_personalization_media IS
    'Immutable ownership and retention link between an order line and each private customer original stored in S3-compatible storage.';
