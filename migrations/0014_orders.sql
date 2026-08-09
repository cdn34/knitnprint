CREATE SEQUENCE order_number_sequence;

CREATE TABLE orders (
    id uuid PRIMARY KEY,
    order_number text NOT NULL UNIQUE,
    cart_id uuid NOT NULL UNIQUE REFERENCES carts(id) ON DELETE RESTRICT,
    customer_id uuid REFERENCES customers(id) ON DELETE SET NULL,
    checkout_idempotency_hash bytea NOT NULL,
    order_status text NOT NULL DEFAULT 'pending',
    payment_status text NOT NULL DEFAULT 'pending',
    fulfillment_status text NOT NULL DEFAULT 'unfulfilled',
    currency char(3) NOT NULL,
    subtotal_minor bigint NOT NULL,
    discount_minor bigint NOT NULL DEFAULT 0,
    shipping_minor bigint NOT NULL DEFAULT 0,
    tax_minor bigint NOT NULL DEFAULT 0,
    total_minor bigint NOT NULL,
    customer_email text NOT NULL,
    customer_first_name text NOT NULL,
    customer_last_name text NOT NULL,
    customer_phone text NOT NULL DEFAULT '',
    shipping_recipient_name text NOT NULL,
    shipping_line1 text NOT NULL,
    shipping_line2 text NOT NULL DEFAULT '',
    shipping_city text NOT NULL,
    shipping_region text NOT NULL DEFAULT '',
    shipping_postal_code text NOT NULL,
    shipping_country_code char(2) NOT NULL,
    shipping_phone text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT orders_idempotency_hash_length CHECK (octet_length(checkout_idempotency_hash) = 32),
    CONSTRAINT orders_order_status_valid CHECK (order_status IN ('pending', 'confirmed', 'completed', 'cancelled')),
    CONSTRAINT orders_payment_status_valid CHECK (payment_status IN ('pending', 'authorized', 'paid', 'refunded', 'failed')),
    CONSTRAINT orders_fulfillment_status_valid CHECK (fulfillment_status IN ('unfulfilled', 'fulfilled')),
    CONSTRAINT orders_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT orders_country_uppercase CHECK (shipping_country_code ~ '^[A-Z]{2}$'),
    CONSTRAINT orders_amounts_non_negative CHECK (
        subtotal_minor >= 0 AND discount_minor >= 0 AND shipping_minor >= 0
        AND tax_minor >= 0 AND total_minor >= 0
    ),
    CONSTRAINT orders_total_consistent CHECK (
        total_minor = subtotal_minor - discount_minor + shipping_minor + tax_minor
    )
);

CREATE INDEX orders_created ON orders (created_at DESC, id DESC);
CREATE INDEX orders_customer_history ON orders (customer_id, created_at DESC)
    WHERE customer_id IS NOT NULL;

CREATE TABLE order_lines (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    product_id uuid REFERENCES products(id) ON DELETE SET NULL,
    variant_id uuid REFERENCES product_variants(id) ON DELETE SET NULL,
    product_title text NOT NULL,
    variant_title text NOT NULL,
    sku text NOT NULL,
    quantity integer NOT NULL,
    unit_price_minor bigint NOT NULL,
    line_total_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    position integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (order_id, position),
    CONSTRAINT order_lines_quantity_positive CHECK (quantity BETWEEN 1 AND 99),
    CONSTRAINT order_lines_amounts_non_negative CHECK (unit_price_minor >= 0 AND line_total_minor >= 0),
    CONSTRAINT order_lines_total_consistent CHECK (line_total_minor = unit_price_minor * quantity),
    CONSTRAINT order_lines_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE TABLE order_payments (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL UNIQUE REFERENCES orders(id) ON DELETE RESTRICT,
    provider text NOT NULL,
    status text NOT NULL DEFAULT 'pending',
    amount_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    paid_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT order_payments_provider_valid CHECK (provider IN ('manual', 'stripe')),
    CONSTRAINT order_payments_status_valid CHECK (status IN ('pending', 'authorized', 'paid', 'refunded', 'failed')),
    CONSTRAINT order_payments_amount_non_negative CHECK (amount_minor >= 0),
    CONSTRAINT order_payments_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT order_payments_paid_at_consistent CHECK ((status = 'paid') = (paid_at IS NOT NULL))
);

CREATE TABLE order_events (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    actor_customer_id uuid REFERENCES customers(id) ON DELETE SET NULL,
    event_type text NOT NULL,
    title text NOT NULL,
    detail text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT order_events_actor_exclusive CHECK (
        actor_staff_user_id IS NULL OR actor_customer_id IS NULL
    ),
    CONSTRAINT order_events_type_not_blank CHECK (length(btrim(event_type)) BETWEEN 3 AND 100),
    CONSTRAINT order_events_title_not_blank CHECK (length(btrim(title)) BETWEEN 3 AND 200)
);

CREATE INDEX order_events_timeline ON order_events (order_id, created_at, id);

CREATE FUNCTION prevent_order_snapshot_changes() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'order commercial snapshots are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER order_lines_immutable
BEFORE UPDATE OR DELETE ON order_lines
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

CREATE TRIGGER order_events_immutable
BEFORE UPDATE OR DELETE ON order_events
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

COMMENT ON TABLE orders IS 'Orders retain immutable commercial and delivery snapshots independent of mutable source records.';
COMMENT ON TABLE order_events IS 'Append-only customer and staff-visible order timeline.';
