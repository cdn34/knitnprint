ALTER TABLE orders ADD COLUMN locale text NOT NULL DEFAULT 'en'
    CHECK (locale IN ('en', 'pt', 'es'));

-- Keep flattened email previews separate from customer originals used in production.
ALTER TABLE cart_lines ADD COLUMN confirmation_preview_media_asset_ids uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE order_lines ADD COLUMN confirmation_preview_media_asset_ids uuid[] NOT NULL DEFAULT '{}';

COMMENT ON COLUMN orders.locale IS 'Language selected at checkout, retained for delayed transactional email delivery.';

CREATE FUNCTION prevent_order_locale_changes() RETURNS trigger AS $$
BEGIN
    IF NEW.locale IS DISTINCT FROM OLD.locale THEN
        RAISE EXCEPTION 'order language snapshot is immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER orders_locale_immutable BEFORE UPDATE OF locale ON orders
FOR EACH ROW EXECUTE FUNCTION prevent_order_locale_changes();
