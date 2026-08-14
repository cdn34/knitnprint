CREATE TABLE store_settings (
    singleton boolean PRIMARY KEY DEFAULT true,
    store_name text NOT NULL,
    support_email text NOT NULL,
    currency char(3) NOT NULL,
    tax_enabled boolean NOT NULL DEFAULT false,
    updated_by_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT store_settings_singleton CHECK (singleton),
    CONSTRAINT store_settings_name_valid CHECK (length(btrim(store_name)) BETWEEN 2 AND 100),
    CONSTRAINT store_settings_email_valid CHECK (length(btrim(support_email)) BETWEEN 3 AND 320),
    CONSTRAINT store_settings_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$')
);

INSERT INTO store_settings (store_name, support_email, currency)
VALUES ('KnitPrint', 'hello@knitprint.local', 'EUR');

CREATE TABLE shipping_zones (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    country_codes text[] NOT NULL DEFAULT '{}',
    priority integer NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT shipping_zones_name_valid CHECK (length(btrim(name)) BETWEEN 2 AND 100),
    CONSTRAINT shipping_zones_countries_bounded CHECK (cardinality(country_codes) BETWEEN 0 AND 249),
    CONSTRAINT shipping_zones_priority_non_negative CHECK (priority >= 0),
    UNIQUE (priority)
);

CREATE TABLE shipping_methods (
    id uuid PRIMARY KEY,
    shipping_zone_id uuid NOT NULL REFERENCES shipping_zones(id) ON DELETE CASCADE,
    name text NOT NULL,
    flat_rate_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    position integer NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT shipping_methods_name_valid CHECK (length(btrim(name)) BETWEEN 2 AND 100),
    CONSTRAINT shipping_methods_rate_non_negative CHECK (flat_rate_minor >= 0),
    CONSTRAINT shipping_methods_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT shipping_methods_position_non_negative CHECK (position >= 0),
    UNIQUE (shipping_zone_id, position)
);

CREATE INDEX shipping_zones_matching ON shipping_zones (active, priority);
CREATE INDEX shipping_methods_zone ON shipping_methods (shipping_zone_id, active, position);

INSERT INTO shipping_zones (id, name, priority)
VALUES ('00000000-0000-7000-8000-000000000011', 'Worldwide', 0);

INSERT INTO shipping_methods (id, shipping_zone_id, name, flat_rate_minor, currency, position)
VALUES (
    '00000000-0000-7000-8000-000000000012',
    '00000000-0000-7000-8000-000000000011',
    'Standard shipping',
    0,
    'EUR',
    0
);

CREATE TABLE tax_rules (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    country_codes text[] NOT NULL DEFAULT '{}',
    rate_basis_points integer NOT NULL,
    priority integer NOT NULL,
    active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tax_rules_name_valid CHECK (length(btrim(name)) BETWEEN 2 AND 100),
    CONSTRAINT tax_rules_countries_bounded CHECK (cardinality(country_codes) BETWEEN 0 AND 249),
    CONSTRAINT tax_rules_rate_valid CHECK (rate_basis_points BETWEEN 0 AND 10000),
    CONSTRAINT tax_rules_priority_non_negative CHECK (priority >= 0),
    UNIQUE (priority)
);

CREATE INDEX tax_rules_matching ON tax_rules (active, priority);

ALTER TABLE carts
ADD COLUMN shipping_method_id uuid REFERENCES shipping_methods(id) ON DELETE SET NULL;

CREATE INDEX carts_shipping_method ON carts (shipping_method_id)
WHERE shipping_method_id IS NOT NULL;

CREATE TABLE order_shipping_snapshots (
    order_id uuid PRIMARY KEY REFERENCES orders(id) ON DELETE RESTRICT,
    shipping_method_id uuid,
    zone_name text NOT NULL,
    method_name text NOT NULL,
    country_code char(2) NOT NULL,
    amount_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT order_shipping_zone_valid CHECK (length(btrim(zone_name)) BETWEEN 2 AND 100),
    CONSTRAINT order_shipping_method_valid CHECK (length(btrim(method_name)) BETWEEN 2 AND 100),
    CONSTRAINT order_shipping_country_uppercase CHECK (country_code ~ '^[A-Z]{2}$'),
    CONSTRAINT order_shipping_amount_non_negative CHECK (amount_minor >= 0),
    CONSTRAINT order_shipping_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE TABLE order_tax_snapshots (
    order_id uuid PRIMARY KEY REFERENCES orders(id) ON DELETE RESTRICT,
    tax_rule_id uuid,
    rule_name text NOT NULL,
    country_code char(2) NOT NULL,
    rate_basis_points integer NOT NULL,
    taxable_amount_minor bigint NOT NULL,
    amount_minor bigint NOT NULL,
    behavior text NOT NULL DEFAULT 'exclusive',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT order_tax_rule_valid CHECK (length(btrim(rule_name)) BETWEEN 2 AND 100),
    CONSTRAINT order_tax_country_uppercase CHECK (country_code ~ '^[A-Z]{2}$'),
    CONSTRAINT order_tax_rate_valid CHECK (rate_basis_points BETWEEN 0 AND 10000),
    CONSTRAINT order_tax_amounts_non_negative CHECK (taxable_amount_minor >= 0 AND amount_minor >= 0),
    CONSTRAINT order_tax_behavior_valid CHECK (behavior IN ('exclusive', 'disabled', 'no_matching_rule'))
);

CREATE TABLE settings_history (
    id uuid PRIMARY KEY,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    reason text NOT NULL,
    snapshot jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT settings_history_reason_valid CHECK (length(btrim(reason)) BETWEEN 3 AND 500),
    CONSTRAINT settings_history_snapshot_object CHECK (jsonb_typeof(snapshot) = 'object')
);

CREATE INDEX settings_history_recent ON settings_history (created_at DESC, id DESC);

INSERT INTO order_shipping_snapshots (
    order_id, zone_name, method_name, country_code, amount_minor, currency
)
SELECT id, 'Legacy orders', 'Recorded shipping', shipping_country_code,
       shipping_minor, currency
FROM orders;

INSERT INTO order_tax_snapshots (
    order_id, rule_name, country_code, rate_basis_points,
    taxable_amount_minor, amount_minor, behavior
)
SELECT id, 'Recorded tax', shipping_country_code, 0,
       subtotal_minor - discount_minor + shipping_minor, tax_minor,
       CASE WHEN tax_minor = 0 THEN 'disabled' ELSE 'exclusive' END
FROM orders;

CREATE TRIGGER order_shipping_snapshots_immutable
BEFORE UPDATE OR DELETE ON order_shipping_snapshots
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

CREATE TRIGGER order_tax_snapshots_immutable
BEFORE UPDATE OR DELETE ON order_tax_snapshots
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

CREATE FUNCTION prevent_settings_history_changes() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'settings history is immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER settings_history_immutable
BEFORE UPDATE OR DELETE ON settings_history
FOR EACH ROW EXECUTE FUNCTION prevent_settings_history_changes();

COMMENT ON TABLE store_settings IS 'Singleton non-secret commercial identity and pricing configuration.';
COMMENT ON TABLE shipping_zones IS 'Ordered destination matches; an empty country list is the worldwide fallback.';
COMMENT ON TABLE tax_rules IS 'Jurisdiction-neutral destination rates configured only after tax requirements are confirmed.';
COMMENT ON TABLE order_shipping_snapshots IS 'Immutable shipping rule and amount retained with an order.';
COMMENT ON TABLE order_tax_snapshots IS 'Immutable tax behavior, rule, base, and amount retained with an order.';
COMMENT ON TABLE settings_history IS 'Append-only staff-attributed commercial settings snapshots.';
