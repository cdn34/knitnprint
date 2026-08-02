CREATE TABLE guest_customer_requests (
    idempotency_hash bytea PRIMARY KEY,
    customer_id uuid NOT NULL UNIQUE REFERENCES customers(id) ON DELETE CASCADE,
    address_id uuid NOT NULL UNIQUE REFERENCES customer_addresses(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT guest_customer_requests_hash_length CHECK (octet_length(idempotency_hash) = 32)
);

COMMENT ON TABLE guest_customer_requests IS
    'Stores only hashed idempotency keys so guest-capture retries return one customer and address.';
