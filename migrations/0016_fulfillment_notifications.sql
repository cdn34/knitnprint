ALTER TABLE orders DROP CONSTRAINT orders_fulfillment_status_valid;
ALTER TABLE orders ADD CONSTRAINT orders_fulfillment_status_valid
    CHECK (fulfillment_status IN ('unfulfilled', 'partially_fulfilled', 'fulfilled'));

CREATE TABLE fulfillments (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    idempotency_hash bytea NOT NULL,
    request_hash bytea NOT NULL,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    carrier text NOT NULL DEFAULT '',
    tracking_number text NOT NULL DEFAULT '',
    tracking_url text NOT NULL DEFAULT '',
    reason text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (order_id, idempotency_hash),
    CONSTRAINT fulfillments_idempotency_hash_length CHECK (octet_length(idempotency_hash) = 32),
    CONSTRAINT fulfillments_request_hash_length CHECK (octet_length(request_hash) = 32),
    CONSTRAINT fulfillments_carrier_length CHECK (length(carrier) <= 100),
    CONSTRAINT fulfillments_tracking_number_length CHECK (length(tracking_number) <= 200),
    CONSTRAINT fulfillments_tracking_url_valid CHECK (
        tracking_url = '' OR (length(tracking_url) <= 2000 AND tracking_url ~ '^https://')
    ),
    CONSTRAINT fulfillments_reason_length CHECK (length(btrim(reason)) BETWEEN 3 AND 500)
);

CREATE INDEX fulfillments_order_history ON fulfillments (order_id, created_at, id);

CREATE TABLE fulfillment_lines (
    fulfillment_id uuid NOT NULL REFERENCES fulfillments(id) ON DELETE RESTRICT,
    order_line_id uuid NOT NULL REFERENCES order_lines(id) ON DELETE RESTRICT,
    quantity integer NOT NULL,
    PRIMARY KEY (fulfillment_id, order_line_id),
    CONSTRAINT fulfillment_lines_quantity_positive CHECK (quantity BETWEEN 1 AND 99)
);

CREATE TABLE notification_jobs (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    fulfillment_id uuid REFERENCES fulfillments(id) ON DELETE RESTRICT,
    kind text NOT NULL,
    deduplication_key text NOT NULL,
    recipient_email text NOT NULL,
    status text NOT NULL DEFAULT 'pending',
    attempt_count integer NOT NULL DEFAULT 0,
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    locked_at timestamptz,
    sent_at timestamptz,
    last_error text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (kind, deduplication_key),
    CONSTRAINT notification_jobs_kind_valid CHECK (kind IN ('order_confirmation', 'fulfillment_created')),
    CONSTRAINT notification_jobs_status_valid CHECK (status IN ('pending', 'processing', 'sent', 'failed')),
    CONSTRAINT notification_jobs_attempts_non_negative CHECK (attempt_count >= 0),
    CONSTRAINT notification_jobs_recipient_not_blank CHECK (length(btrim(recipient_email)) BETWEEN 3 AND 320),
    CONSTRAINT notification_jobs_sent_consistent CHECK ((status = 'sent') = (sent_at IS NOT NULL)),
    CONSTRAINT notification_jobs_fulfillment_consistent CHECK (
        (kind = 'order_confirmation' AND fulfillment_id IS NULL)
        OR (kind = 'fulfillment_created' AND fulfillment_id IS NOT NULL)
    )
);

CREATE INDEX notification_jobs_due
    ON notification_jobs (next_attempt_at, id)
    WHERE status IN ('pending', 'processing');

CREATE TABLE notification_attempts (
    id uuid PRIMARY KEY,
    notification_job_id uuid NOT NULL REFERENCES notification_jobs(id) ON DELETE RESTRICT,
    attempt_number integer NOT NULL,
    outcome text NOT NULL,
    error_message text,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (notification_job_id, attempt_number),
    CONSTRAINT notification_attempts_number_positive CHECK (attempt_number > 0),
    CONSTRAINT notification_attempts_outcome_valid CHECK (outcome IN ('sent', 'failed'))
);

CREATE TRIGGER fulfillments_immutable
BEFORE UPDATE OR DELETE ON fulfillments
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

CREATE TRIGGER fulfillment_lines_immutable
BEFORE UPDATE OR DELETE ON fulfillment_lines
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

CREATE TRIGGER notification_attempts_immutable
BEFORE UPDATE OR DELETE ON notification_attempts
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

COMMENT ON TABLE fulfillments IS 'Immutable complete or partial shipment records created by authorized staff.';
COMMENT ON TABLE notification_jobs IS 'Durable idempotent email outbox; delivery failure never rolls back commercial state.';
COMMENT ON TABLE notification_attempts IS 'Append-only history of email delivery outcomes.';
