CREATE SEQUENCE catalog_sku_sequence;
CREATE TABLE catalog_skus (sku text PRIMARY KEY, owner_id uuid NOT NULL UNIQUE);
INSERT INTO catalog_skus SELECT sku,id FROM product_variants;
CREATE FUNCTION allocate_product_sku(owner uuid) RETURNS text AS $$
DECLARE candidate text;
BEGIN
 SELECT sku INTO candidate FROM catalog_skus WHERE owner_id=owner;
 IF FOUND THEN RETURN candidate; END IF;
 LOOP
  candidate := nextval('catalog_sku_sequence')::text;
  candidate := 'KP-' || lpad(candidate,greatest(6,length(candidate)),'0');
  INSERT INTO catalog_skus VALUES(candidate,owner) ON CONFLICT DO NOTHING;
  IF FOUND THEN RETURN candidate; END IF;
 END LOOP;
END; $$ LANGUAGE plpgsql;
ALTER TABLE products ADD COLUMN sku text;
UPDATE products SET sku=allocate_product_sku(id);
ALTER TABLE products ALTER COLUMN sku SET NOT NULL;
ALTER TABLE products ADD COLUMN variant_config jsonb NOT NULL DEFAULT '{"options":[],"color_photos":false}'::jsonb;
ALTER TABLE products ADD COLUMN variant_revision bigint NOT NULL DEFAULT 0;
ALTER TABLE product_variants ADD COLUMN selections jsonb;
ALTER TABLE product_variants ADD COLUMN retired boolean NOT NULL DEFAULT false;
CREATE UNIQUE INDEX product_variant_combination ON product_variants(product_id,selections) WHERE selections IS NOT NULL;
CREATE FUNCTION product_sku_identity() RETURNS trigger AS $$
BEGIN
 IF TG_OP='INSERT' THEN NEW.sku:=allocate_product_sku(NEW.id);
 ELSIF NEW.sku IS DISTINCT FROM OLD.sku THEN RAISE EXCEPTION 'Product SKU is immutable'; END IF;
 RETURN NEW;
END; $$ LANGUAGE plpgsql;
CREATE TRIGGER product_sku_identity BEFORE INSERT OR UPDATE ON products FOR EACH ROW EXECUTE FUNCTION product_sku_identity();
CREATE FUNCTION variant_sku_identity() RETURNS trigger AS $$
DECLARE base text; suffix text; candidate text; attempt integer:=0;
BEGIN
 IF TG_OP='UPDATE' THEN
  IF NEW.sku IS DISTINCT FROM OLD.sku OR NEW.product_id IS DISTINCT FROM OLD.product_id OR (OLD.selections IS NOT NULL AND NEW.selections IS DISTINCT FROM OLD.selections) THEN RAISE EXCEPTION 'Variant identity is immutable'; END IF;
  RETURN NEW;
 END IF;
 IF NEW.sku IS NOT NULL AND NEW.sku NOT IN ('','generated') THEN
 INSERT INTO catalog_skus VALUES(NEW.sku,NEW.id); RETURN NEW;
 END IF;
 SELECT sku INTO candidate FROM catalog_skus WHERE owner_id=NEW.id;
 IF FOUND THEN NEW.sku:=candidate; RETURN NEW; END IF;
 SELECT sku INTO STRICT base FROM products WHERE id=NEW.product_id;
 SELECT string_agg(key || '-' || value,'-' ORDER BY key) INTO suffix FROM jsonb_each_text(COALESCE(NEW.selections,'{}'));
 candidate:=base || '-' || COALESCE(suffix,'STD');
 LOOP
  INSERT INTO catalog_skus VALUES(candidate,NEW.id) ON CONFLICT DO NOTHING;
  EXIT WHEN FOUND;
  attempt:=attempt+1; candidate:=base || '-' || COALESCE(suffix,'STD') || '-' || attempt;
 END LOOP;
 NEW.sku:=candidate;
 RETURN NEW;
END; $$ LANGUAGE plpgsql;
CREATE TRIGGER variant_sku_identity BEFORE INSERT OR UPDATE ON product_variants FOR EACH ROW EXECUTE FUNCTION variant_sku_identity();
CREATE FUNCTION protect_catalog_skus() RETURNS trigger AS $$ BEGIN RAISE EXCEPTION 'SKU reservations are permanent'; END; $$ LANGUAGE plpgsql;
CREATE TRIGGER catalog_skus_immutable BEFORE UPDATE OR DELETE ON catalog_skus FOR EACH ROW EXECUTE FUNCTION protect_catalog_skus();
CREATE TABLE variant_templates (id uuid PRIMARY KEY, definition jsonb NOT NULL, revision bigint NOT NULL DEFAULT 0);
INSERT INTO variant_templates VALUES
('10000000-0000-0000-0000-000000000001','{"code":"COL","label":"Color","control":"swatch","values":[{"code":"BLK","label":"Black","color":"#111111"},{"code":"WHT","label":"White","color":"#ffffff"},{"code":"RED","label":"Red","color":"#d32f2f"},{"code":"GRN","label":"Green","color":"#388e3c"},{"code":"BLU","label":"Blue","color":"#1976d2"},{"code":"YEL","label":"Yellow","color":"#fbc02d"},{"code":"PNK","label":"Pink","color":"#ec8fae"},{"code":"GRY","label":"Grey","color":"#808080"}]}'),
('10000000-0000-0000-0000-000000000002','{"code":"SIZ","label":"Size","control":"buttons","values":[{"code":"XS","label":"XS"},{"code":"S","label":"S"},{"code":"M","label":"M"},{"code":"L","label":"L"},{"code":"XL","label":"XL"},{"code":"XXL","label":"XXL"}]}'),
('10000000-0000-0000-0000-000000000003','{"code":"SIZ","label":"Size (months)","control":"buttons","values":[{"code":"M06","label":"0–6"},{"code":"M612","label":"6–12"},{"code":"M1218","label":"12–18"},{"code":"M1824","label":"18–24"}]}'),
('10000000-0000-0000-0000-000000000004','{"code":"SIZ","label":"Size (years)","control":"buttons","values":[{"code":"Y12","label":"1–2"},{"code":"Y24","label":"2–4"},{"code":"Y56","label":"5–6"}]}');
CREATE FUNCTION variant_photo_ids(config jsonb, selected jsonb) RETURNS SETOF text AS $$
 SELECT photo #>> '{}' FROM jsonb_array_elements(COALESCE(config->'options','[]')) opt,
 jsonb_array_elements(COALESCE(opt->'values','[]')) val,
 jsonb_array_elements(COALESCE(val->'media_ids','[]')) photo
 WHERE config->>'color_photos'='true' AND opt->>'control'='swatch' AND selected->>(opt->>'code')=val->>'code';
$$ LANGUAGE sql IMMUTABLE;
CREATE FUNCTION variant_views(config jsonb, selected jsonb, views jsonb) RETURNS jsonb AS $$
 SELECT CASE WHEN EXISTS(SELECT 1 FROM variant_photo_ids(config,selected)) THEN
 COALESCE((SELECT jsonb_agg(view) FROM jsonb_array_elements(views) view WHERE view->>'media_id' IN (SELECT variant_photo_ids(config,selected))),'[]'::jsonb)
 ELSE views END;
$$ LANGUAGE sql IMMUTABLE;
