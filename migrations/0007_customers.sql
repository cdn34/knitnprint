CREATE TABLE customers (
    id uuid PRIMARY KEY,
    customer_type text NOT NULL DEFAULT 'guest',
    email citext NOT NULL,
    first_name text NOT NULL,
    last_name text NOT NULL,
    phone text NOT NULL DEFAULT '',
    retention_expires_at timestamptz NOT NULL DEFAULT (now() + interval '24 months'),
    anonymized_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    search_document tsvector GENERATED ALWAYS AS (
        to_tsvector(
            'simple',
            email::text || ' ' || first_name || ' ' || last_name || ' ' || phone
        )
    ) STORED,
    CONSTRAINT customers_type_valid CHECK (customer_type IN ('guest', 'registered')),
    CONSTRAINT customers_email_not_blank CHECK (length(btrim(email::text)) BETWEEN 3 AND 320),
    CONSTRAINT customers_first_name_valid CHECK (length(btrim(first_name)) BETWEEN 1 AND 100),
    CONSTRAINT customers_last_name_valid CHECK (length(btrim(last_name)) BETWEEN 1 AND 100),
    CONSTRAINT customers_phone_length CHECK (length(phone) <= 40),
    CONSTRAINT customers_retention_after_creation CHECK (retention_expires_at > created_at)
);

-- Supports staff lookup by normalized email and the bounded customer search API.
CREATE INDEX customers_email_lookup ON customers (email);
CREATE INDEX customers_search ON customers USING gin (search_document);
CREATE INDEX customers_active_created
    ON customers (created_at DESC, id)
    WHERE anonymized_at IS NULL;

CREATE TABLE customer_addresses (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    address_type text NOT NULL DEFAULT 'delivery',
    recipient_name text NOT NULL,
    line1 text NOT NULL,
    line2 text NOT NULL DEFAULT '',
    city text NOT NULL,
    region text NOT NULL DEFAULT '',
    postal_code text NOT NULL,
    country_code char(2) NOT NULL,
    phone text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT customer_addresses_type_valid CHECK (address_type IN ('delivery', 'billing')),
    CONSTRAINT customer_addresses_recipient_valid CHECK (length(btrim(recipient_name)) BETWEEN 1 AND 200),
    CONSTRAINT customer_addresses_line1_valid CHECK (length(btrim(line1)) BETWEEN 1 AND 200),
    CONSTRAINT customer_addresses_line2_length CHECK (length(line2) <= 200),
    CONSTRAINT customer_addresses_city_valid CHECK (length(btrim(city)) BETWEEN 1 AND 120),
    CONSTRAINT customer_addresses_region_length CHECK (length(region) <= 120),
    CONSTRAINT customer_addresses_postal_valid CHECK (length(btrim(postal_code)) BETWEEN 1 AND 32),
    CONSTRAINT customer_addresses_country_valid CHECK (country_code ~ '^[A-Z]{2}$'),
    CONSTRAINT customer_addresses_phone_length CHECK (length(phone) <= 40)
);

-- Supports loading every address on the permission-protected customer detail page.
CREATE INDEX customer_addresses_customer_order
    ON customer_addresses (customer_id, created_at, id);

COMMENT ON COLUMN customers.retention_expires_at IS
    'After this deadline the record is excluded from application reads and is eligible for anonymization.';
COMMENT ON COLUMN customers.anonymized_at IS
    'Marks personal data as unavailable; anonymization cleanup is introduced before production customer traffic.';
