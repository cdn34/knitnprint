ALTER TABLE media_assets
    ADD COLUMN personalization_cart_id uuid REFERENCES carts(id) ON DELETE SET NULL;

WITH single_cart_ownership AS (
    SELECT media_id, (array_agg(DISTINCT line.cart_id))[1] AS cart_id
    FROM cart_lines AS line
    CROSS JOIN LATERAL unnest(line.customization_media_asset_ids) AS media_id
    GROUP BY media_id
    HAVING count(DISTINCT line.cart_id) = 1
)
UPDATE media_assets AS media
SET personalization_cart_id = ownership.cart_id
FROM single_cart_ownership AS ownership
WHERE media.id = ownership.media_id;

CREATE INDEX media_assets_personalization_cart
    ON media_assets (personalization_cart_id, id)
    WHERE personalization_cart_id IS NOT NULL;

COMMENT ON COLUMN media_assets.personalization_cart_id IS
    'Cart session that initiated a customer personalization upload; prevents cross-cart media reuse.';
