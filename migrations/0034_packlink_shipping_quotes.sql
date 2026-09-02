ALTER TABLE carts
ADD COLUMN shipping_quote_id uuid;

CREATE TABLE cart_shipping_quotes (
    id uuid PRIMARY KEY,
    cart_id uuid NOT NULL REFERENCES carts(id) ON DELETE CASCADE,
    provider text NOT NULL DEFAULT 'packlink',
    service_id text NOT NULL,
    carrier_name text NOT NULL,
    service_name text NOT NULL,
    amount_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    departure_dropoff boolean NOT NULL DEFAULT false,
    destination_dropoff boolean NOT NULL DEFAULT false,
    transit_hours integer NOT NULL DEFAULT 0,
    request_hash bytea NOT NULL,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT cart_shipping_quotes_provider_valid CHECK (provider = 'packlink'),
    CONSTRAINT cart_shipping_quotes_service_valid CHECK (length(btrim(service_id)) BETWEEN 1 AND 100),
    CONSTRAINT cart_shipping_quotes_carrier_valid CHECK (length(btrim(carrier_name)) BETWEEN 1 AND 100),
    CONSTRAINT cart_shipping_quotes_name_valid CHECK (length(btrim(service_name)) BETWEEN 1 AND 160),
    CONSTRAINT cart_shipping_quotes_amount_valid CHECK (amount_minor >= 0),
    CONSTRAINT cart_shipping_quotes_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT cart_shipping_quotes_transit_valid CHECK (transit_hours BETWEEN 0 AND 8760),
    CONSTRAINT cart_shipping_quotes_hash_length CHECK (octet_length(request_hash) = 32),
    UNIQUE (cart_id, service_id, departure_dropoff, destination_dropoff)
);

CREATE INDEX cart_shipping_quotes_active
    ON cart_shipping_quotes (cart_id, expires_at, amount_minor, id);

ALTER TABLE carts
ADD CONSTRAINT carts_shipping_quote_fk
FOREIGN KEY (shipping_quote_id) REFERENCES cart_shipping_quotes(id) ON DELETE SET NULL;

CREATE INDEX carts_shipping_quote ON carts (shipping_quote_id)
WHERE shipping_quote_id IS NOT NULL;

ALTER TABLE order_shipping_snapshots
ADD COLUMN provider text NOT NULL DEFAULT 'manual',
ADD COLUMN carrier_name text NOT NULL DEFAULT '',
ADD COLUMN external_service_id text NOT NULL DEFAULT '',
ADD COLUMN departure_dropoff boolean NOT NULL DEFAULT false,
ADD COLUMN destination_dropoff boolean NOT NULL DEFAULT false,
ADD COLUMN transit_hours integer NOT NULL DEFAULT 0;

ALTER TABLE order_shipping_snapshots
ADD CONSTRAINT order_shipping_provider_valid CHECK (provider IN ('manual', 'packlink')),
ADD CONSTRAINT order_shipping_transit_valid CHECK (transit_hours BETWEEN 0 AND 8760);

ALTER TABLE order_shipping_snapshots
DROP CONSTRAINT order_shipping_method_valid,
ADD CONSTRAINT order_shipping_method_valid
CHECK (length(btrim(method_name)) BETWEEN 2 AND 320);

COMMENT ON TABLE cart_shipping_quotes IS
    'Short-lived, server-priced Packlink services. The browser selects only the opaque quote id.';
COMMENT ON COLUMN carts.shipping_quote_id IS
    'Selected dynamic carrier quote. Mutually exclusive with shipping_method_id while Packlink is enabled.';
