-- New prices and configuration must use EUR. NOT VALID preserves existing
-- records without reinterpreting amounts in another currency; PostgreSQL still
-- enforces these constraints on every new or updated row.
ALTER TABLE store_settings
    ADD CONSTRAINT store_settings_currency_eur CHECK (currency = 'EUR') NOT VALID;
ALTER TABLE product_variants
    ADD CONSTRAINT product_variants_currency_eur CHECK (currency = 'EUR') NOT VALID;
ALTER TABLE discounts
    ADD CONSTRAINT discounts_currency_eur CHECK (currency = 'EUR') NOT VALID;
