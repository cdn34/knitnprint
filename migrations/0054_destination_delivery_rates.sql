CREATE TABLE delivery_pricing_config (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    enabled boolean NOT NULL DEFAULT false,
    currency char(3) NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    updated_at timestamptz NOT NULL DEFAULT now(),
    updated_by_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL
);
INSERT INTO delivery_pricing_config (singleton,currency) SELECT true,currency FROM store_settings WHERE singleton;

CREATE TABLE delivery_rates (
    id uuid PRIMARY KEY,
    country_code char(2) NOT NULL CHECK (country_code ~ '^[A-Z]{2}$'),
    region_name text NOT NULL CHECK (length(btrim(region_name)) BETWEEN 2 AND 100),
    postal_prefixes text[] NOT NULL DEFAULT '{}',
    amount_minor bigint CHECK (amount_minor BETWEEN 0 AND 10000000),
    active boolean NOT NULL DEFAULT false,
    position integer NOT NULL CHECK (position >= 0),
    CHECK (cardinality(postal_prefixes) <= 100),
    CHECK (NOT active OR amount_minor IS NOT NULL)
);

INSERT INTO delivery_rates (id,country_code,region_name,postal_prefixes,position) VALUES
('00000000-0000-4000-8000-000000000001','PT','Mainland Portugal',ARRAY['1','2','3','4','5','6','7','8'],0),
('00000000-0000-4000-8000-000000000002','PT','Madeira',ARRAY['90','91','92','93','94'],1),
('00000000-0000-4000-8000-000000000003','PT','Azores',ARRAY['95','96','97','98','99'],2),
('00000000-0000-4000-8000-000000000004','ES','All regions',ARRAY[]::text[],3);

COMMENT ON TABLE delivery_rates IS 'Editable flat charge per order. More specific postal prefixes override country-wide rules, including inactive regional exclusions.';
COMMENT ON TABLE delivery_pricing_config IS 'Explicit opt-in preserves zero shipping until staff configure and activate destination rates.';
