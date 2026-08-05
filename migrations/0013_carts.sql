CREATE TABLE carts (
    id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE,
    customer_id uuid REFERENCES customers(id) ON DELETE SET NULL,
    shipping_address_id uuid REFERENCES customer_addresses(id) ON DELETE SET NULL,
    status text NOT NULL DEFAULT 'active',
    currency char(3),
    expires_at timestamptz NOT NULL DEFAULT (now() + interval '30 days'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT carts_token_hash_length CHECK (octet_length(token_hash) = 32),
    CONSTRAINT carts_status_valid CHECK (status IN ('active', 'converted', 'expired')),
    CONSTRAINT carts_currency_uppercase CHECK (currency IS NULL OR currency ~ '^[A-Z]{3}$'),
    CONSTRAINT carts_expiry_after_creation CHECK (expires_at > created_at)
);

CREATE INDEX carts_customer_activity
    ON carts (customer_id, updated_at DESC)
    WHERE customer_id IS NOT NULL;
CREATE INDEX carts_expiration
    ON carts (expires_at)
    WHERE status = 'active';

CREATE TABLE cart_lines (
    id uuid PRIMARY KEY,
    cart_id uuid NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    variant_id uuid NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    quantity integer NOT NULL,
    unit_price_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (cart_id, variant_id),
    CONSTRAINT cart_lines_quantity_valid CHECK (quantity BETWEEN 1 AND 99),
    CONSTRAINT cart_lines_price_non_negative CHECK (unit_price_minor >= 0),
    CONSTRAINT cart_lines_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE INDEX cart_lines_cart_order ON cart_lines (cart_id, created_at, id);

CREATE TABLE cart_mutations (
    cart_id uuid NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    idempotency_hash bytea NOT NULL,
    request_hash bytea NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (cart_id, idempotency_hash),
    CONSTRAINT cart_mutations_idempotency_hash_length CHECK (octet_length(idempotency_hash) = 32),
    CONSTRAINT cart_mutations_request_hash_length CHECK (octet_length(request_hash) = 32)
);

CREATE INDEX cart_mutations_cleanup ON cart_mutations (created_at);

COMMENT ON TABLE carts IS
    'Disposable checkout preparation state identified by an opaque, hashed browser token.';
COMMENT ON COLUMN cart_lines.unit_price_minor IS
    'Last server-reconciled catalog price; never accepted from a browser request.';
COMMENT ON TABLE cart_mutations IS
    'Hashed idempotency and request fingerprints make retried cart mutations safe.';
