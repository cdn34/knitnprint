DROP INDEX order_refunds_provider_reference;
CREATE UNIQUE INDEX order_refunds_provider_reference
    ON order_refunds (provider, provider_refund_id)
    WHERE provider = 'stripe' AND provider_refund_id IS NOT NULL;
