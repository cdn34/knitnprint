-- Keep every issued product URL attached to its original product. NULL owners
-- are tombstones: deleting a product must never redirect its links to another.
CREATE TABLE product_slug_history (
    slug text PRIMARY KEY,
    product_id uuid REFERENCES products(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX product_slug_history_product ON product_slug_history(product_id);
INSERT INTO product_slug_history(slug, product_id) SELECT slug, id FROM products;

CREATE FUNCTION product_slug_base(input text, parent_sku text) RETURNS text AS $$
DECLARE suffix text := '-' || lower(parent_sku);
BEGIN
    IF right(input,length(suffix)) = suffix THEN
        RETURN left(input,length(input)-length(suffix));
    END IF;
    RETURN input;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE FUNCTION product_slug_with_sku() RETURNS trigger AS $$
DECLARE base text; suffix text; candidate text; attempt integer := 1; modifier text := '';
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.slug = OLD.slug THEN RETURN NEW; END IF;
    base := product_slug_base(NEW.slug,NEW.sku);
    suffix := '-' || lower(NEW.sku);
    -- Reserve the requested legacy URL only if no product has ever owned it.
    INSERT INTO product_slug_history(slug,product_id) VALUES(NEW.slug,NEW.id) ON CONFLICT DO NOTHING;
    LOOP
        candidate := rtrim(left(base,200-length(suffix)-length(modifier)),'-') || modifier || suffix;
        INSERT INTO product_slug_history(slug,product_id) VALUES(candidate,NEW.id) ON CONFLICT DO NOTHING;
        IF FOUND OR EXISTS(SELECT 1 FROM product_slug_history WHERE slug=candidate AND product_id=NEW.id) THEN
            NEW.slug := candidate;
            RETURN NEW;
        END IF;
        attempt := attempt + 1;
        modifier := '-' || attempt;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- BEFORE triggers execute in name order: product_sku_identity allocates the
-- permanent parent SKU before product_slug_with_sku constructs its URL.
CREATE TRIGGER product_slug_with_sku BEFORE INSERT OR UPDATE OF slug ON products
FOR EACH ROW EXECUTE FUNCTION product_slug_with_sku();

-- Force the first canonicalization even though existing slugs are unchanged.
-- Build the candidate here so the trigger's no-change fast path is bypassed.
UPDATE products SET slug=product_slug_base(slug,sku) || '-' || lower(sku)
WHERE right(slug,length(sku)+1) <> '-' || lower(sku);

CREATE FUNCTION protect_product_slug_history() RETURNS trigger AS $$
BEGIN
    IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Product URLs are permanently reserved'; END IF;
    IF NEW.slug IS DISTINCT FROM OLD.slug OR
       (NEW.product_id IS NOT NULL AND NEW.product_id IS DISTINCT FROM OLD.product_id) THEN
        RAISE EXCEPTION 'Product URLs cannot be reassigned';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER product_slug_history_permanent BEFORE UPDATE OR DELETE ON product_slug_history
FOR EACH ROW EXECUTE FUNCTION protect_product_slug_history();
