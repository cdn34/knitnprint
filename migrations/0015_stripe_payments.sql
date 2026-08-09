ALTER TABLE order_payments
    ADD COLUMN provider_payment_id text,
    ADD COLUMN failure_code text,
    ADD COLUMN failure_message text;

CREATE UNIQUE INDEX order_payments_provider_reference
    ON order_payments (provider, provider_payment_id)
    WHERE provider_payment_id IS NOT NULL;

CREATE TABLE payment_attempts (
    id uuid PRIMARY KEY,
    order_payment_id uuid NOT NULL REFERENCES order_payments(id) ON DELETE RESTRICT,
    attempt_number integer NOT NULL,
    provider text NOT NULL,
    provider_payment_id text,
    status text NOT NULL DEFAULT 'creating',
    checkout_url text,
    failure_code text,
    failure_message text,
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (order_payment_id, attempt_number),
    CONSTRAINT payment_attempts_number_positive CHECK (attempt_number > 0),
    CONSTRAINT payment_attempts_provider_valid CHECK (provider IN ('stripe')),
    CONSTRAINT payment_attempts_status_valid CHECK (
        status IN ('creating', 'pending', 'processing', 'succeeded', 'failed', 'cancelled', 'expired')
    ),
    CONSTRAINT payment_attempts_provider_reference_present CHECK (
        status = 'creating' OR provider_payment_id IS NOT NULL
    ),
    CONSTRAINT payment_attempts_checkout_url_present CHECK (
        status <> 'pending' OR checkout_url IS NOT NULL
    )
);

CREATE UNIQUE INDEX payment_attempts_provider_reference
    ON payment_attempts (provider, provider_payment_id)
    WHERE provider_payment_id IS NOT NULL;
CREATE INDEX payment_attempts_abandoned
    ON payment_attempts (expires_at, id)
    WHERE status IN ('creating', 'pending', 'processing');

CREATE TABLE payment_status_events (
    id uuid PRIMARY KEY,
    order_payment_id uuid NOT NULL REFERENCES order_payments(id) ON DELETE RESTRICT,
    payment_attempt_id uuid REFERENCES payment_attempts(id) ON DELETE RESTRICT,
    provider text NOT NULL,
    provider_event_id text,
    event_type text NOT NULL,
    provider_status text NOT NULL,
    detail text NOT NULL DEFAULT '',
    payload_sha256 bytea,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT payment_status_events_provider_valid CHECK (provider IN ('manual', 'stripe')),
    CONSTRAINT payment_status_events_event_not_blank CHECK (length(btrim(event_type)) BETWEEN 3 AND 120),
    CONSTRAINT payment_status_events_status_not_blank CHECK (length(btrim(provider_status)) BETWEEN 3 AND 80),
    CONSTRAINT payment_status_events_payload_hash_length CHECK (
        payload_sha256 IS NULL OR octet_length(payload_sha256) = 32
    )
);

CREATE UNIQUE INDEX payment_status_events_provider_event
    ON payment_status_events (provider, provider_event_id)
    WHERE provider_event_id IS NOT NULL;
CREATE INDEX payment_status_events_history
    ON payment_status_events (order_payment_id, created_at, id);

CREATE TRIGGER payment_status_events_immutable
BEFORE UPDATE OR DELETE ON payment_status_events
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

COMMENT ON TABLE payment_attempts IS 'Retry-safe external payment attempts with stable provider idempotency identifiers.';
COMMENT ON TABLE payment_status_events IS 'Append-only provider payment history; unique provider event IDs make webhook delivery idempotent.';
