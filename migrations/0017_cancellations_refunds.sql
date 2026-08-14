ALTER TABLE orders DROP CONSTRAINT orders_payment_status_valid;
ALTER TABLE orders ADD CONSTRAINT orders_payment_status_valid
    CHECK (payment_status IN (
        'pending', 'authorized', 'paid', 'partially_refunded', 'refunded', 'failed', 'cancelled'
    ));

ALTER TABLE order_payments DROP CONSTRAINT order_payments_status_valid;
ALTER TABLE order_payments ADD CONSTRAINT order_payments_status_valid
    CHECK (status IN (
        'pending', 'authorized', 'paid', 'partially_refunded', 'refunded', 'failed', 'cancelled'
    ));
ALTER TABLE order_payments DROP CONSTRAINT order_payments_paid_at_consistent;
ALTER TABLE order_payments ADD CONSTRAINT order_payments_paid_at_consistent CHECK (
    (status IN ('paid', 'partially_refunded', 'refunded')) = (paid_at IS NOT NULL)
);
ALTER TABLE order_payments ADD COLUMN provider_charge_id text;

ALTER TABLE inventory_movements DROP CONSTRAINT inventory_movement_type_valid;
ALTER TABLE inventory_movements ADD CONSTRAINT inventory_movement_type_valid CHECK (
    movement_type IN ('adjustment', 'reservation', 'release', 'commitment', 'restock')
);

CREATE TABLE order_cancellations (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    idempotency_hash bytea NOT NULL,
    request_hash bytea NOT NULL,
    reason text NOT NULL,
    internal_note text NOT NULL DEFAULT '',
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (order_id, idempotency_hash),
    CONSTRAINT order_cancellations_idempotency_hash_length CHECK (octet_length(idempotency_hash) = 32),
    CONSTRAINT order_cancellations_request_hash_length CHECK (octet_length(request_hash) = 32),
    CONSTRAINT order_cancellations_reason_length CHECK (length(btrim(reason)) BETWEEN 3 AND 500),
    CONSTRAINT order_cancellations_note_length CHECK (length(internal_note) <= 2000)
);

CREATE INDEX order_cancellations_order_history ON order_cancellations (order_id, created_at, id);

CREATE TABLE order_refunds (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    order_payment_id uuid NOT NULL REFERENCES order_payments(id) ON DELETE RESTRICT,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    idempotency_hash bytea NOT NULL,
    request_hash bytea NOT NULL,
    provider text NOT NULL,
    provider_refund_id text,
    status text NOT NULL DEFAULT 'pending',
    mode text NOT NULL,
    amount_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    restock boolean NOT NULL,
    reason text NOT NULL,
    internal_note text NOT NULL DEFAULT '',
    failure_code text,
    failure_message text,
    completed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (order_id, idempotency_hash),
    CONSTRAINT order_refunds_idempotency_hash_length CHECK (octet_length(idempotency_hash) = 32),
    CONSTRAINT order_refunds_request_hash_length CHECK (octet_length(request_hash) = 32),
    CONSTRAINT order_refunds_provider_valid CHECK (provider IN ('manual', 'stripe')),
    CONSTRAINT order_refunds_status_valid CHECK (status IN ('pending', 'succeeded', 'failed')),
    CONSTRAINT order_refunds_mode_valid CHECK (mode IN ('full', 'partial')),
    CONSTRAINT order_refunds_amount_positive CHECK (amount_minor > 0),
    CONSTRAINT order_refunds_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT order_refunds_reason_length CHECK (length(btrim(reason)) BETWEEN 3 AND 500),
    CONSTRAINT order_refunds_note_length CHECK (length(internal_note) <= 2000),
    CONSTRAINT order_refunds_completed_consistent CHECK ((status = 'succeeded') = (completed_at IS NOT NULL))
);

CREATE UNIQUE INDEX order_refunds_provider_reference
    ON order_refunds (provider, provider_refund_id)
    WHERE provider_refund_id IS NOT NULL;
CREATE INDEX order_refunds_order_history ON order_refunds (order_id, created_at, id);

CREATE TABLE order_refund_lines (
    refund_id uuid NOT NULL REFERENCES order_refunds(id) ON DELETE RESTRICT,
    order_line_id uuid NOT NULL REFERENCES order_lines(id) ON DELETE RESTRICT,
    quantity integer NOT NULL,
    amount_minor bigint NOT NULL,
    PRIMARY KEY (refund_id, order_line_id),
    CONSTRAINT order_refund_lines_quantity_positive CHECK (quantity BETWEEN 1 AND 99),
    CONSTRAINT order_refund_lines_amount_non_negative CHECK (amount_minor >= 0)
);

CREATE TRIGGER order_cancellations_immutable
BEFORE UPDATE OR DELETE ON order_cancellations
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

CREATE TRIGGER order_refund_lines_immutable
BEFORE UPDATE OR DELETE ON order_refund_lines
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

COMMENT ON TABLE order_cancellations IS 'Immutable staff cancellation records for unpaid, unfulfilled orders.';
COMMENT ON TABLE order_refunds IS 'Idempotent complete and partial payment refunds with provider outcome history.';
COMMENT ON TABLE order_refund_lines IS 'Immutable server-priced order-line quantities associated with a refund.';
