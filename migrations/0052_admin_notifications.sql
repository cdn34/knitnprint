INSERT INTO capabilities (name, description) VALUES
    ('notifications.read', 'View and search email notification jobs'),
    ('notifications.send', 'Manually deliver pending or failed email notifications');

-- An outbox job need not belong to an order. Each new kind still requires an
-- explicit delivery handler before it can be sent by the API or worker.
ALTER TABLE notification_jobs ALTER COLUMN order_id DROP NOT NULL;
ALTER TABLE notification_jobs DROP CONSTRAINT notification_jobs_kind_valid,
    ADD CONSTRAINT notification_jobs_kind_valid
        CHECK (kind ~ '^[a-z][a-z0-9_]{0,99}$');
ALTER TABLE notification_jobs DROP CONSTRAINT notification_jobs_fulfillment_consistent,
    ADD CONSTRAINT notification_jobs_fulfillment_consistent CHECK (
        (kind = 'fulfillment_created' AND order_id IS NOT NULL AND fulfillment_id IS NOT NULL)
        OR (kind = 'order_confirmation' AND order_id IS NOT NULL AND fulfillment_id IS NULL)
        OR (kind NOT IN ('fulfillment_created', 'order_confirmation') AND fulfillment_id IS NULL)
    );

CREATE INDEX notification_jobs_admin_queue
    ON notification_jobs (status, created_at DESC, id DESC);
