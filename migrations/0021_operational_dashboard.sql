CREATE INDEX orders_paid_fulfillment_queue
ON orders (created_at, id)
WHERE payment_status IN ('paid', 'partially_refunded')
  AND fulfillment_status <> 'fulfilled'
  AND order_status <> 'cancelled';

CREATE INDEX order_payments_failed_recent
ON order_payments (updated_at DESC, id DESC)
WHERE status = 'failed';

CREATE INDEX order_refunds_recent
ON order_refunds (created_at DESC, id DESC);

COMMENT ON INDEX orders_paid_fulfillment_queue IS
    'Supports the operational queue of captured orders that still require fulfillment.';
COMMENT ON INDEX order_payments_failed_recent IS
    'Supports bounded dashboard inspection of currently failed payments.';
COMMENT ON INDEX order_refunds_recent IS
    'Supports bounded cross-order refund activity without changing immutable refund records.';
