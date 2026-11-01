ALTER TABLE orders ADD COLUMN email_locale text NOT NULL DEFAULT 'en'
    CHECK (email_locale IN ('en', 'pt', 'es'));
ALTER TABLE cart_lines ADD COLUMN personalization_previews jsonb NOT NULL DEFAULT '[]'::jsonb
    CHECK (jsonb_typeof(personalization_previews) = 'array');
ALTER TABLE order_lines ADD COLUMN personalization_previews jsonb NOT NULL DEFAULT '[]'::jsonb
    CHECK (jsonb_typeof(personalization_previews) = 'array'),
    ADD COLUMN variant_options jsonb NOT NULL DEFAULT '{}'::jsonb;
COMMENT ON COLUMN order_lines.personalization_previews IS
    'Immutable flattened customer-approved views captured before adding the design to the cart; media retained by order_line_personalization_media.';
-- Carrier notifications replace store fulfillment emails. Suppress outstanding jobs too.
ALTER TABLE notification_jobs DROP CONSTRAINT notification_jobs_status_valid,
    ADD CONSTRAINT notification_jobs_status_valid CHECK (status IN ('pending', 'processing', 'sent', 'failed', 'suppressed'));
UPDATE notification_jobs SET status = 'suppressed', locked_at = NULL,
    last_error = 'Suppressed: shipping updates are sent by the carrier', updated_at = now()
WHERE kind = 'fulfillment_created' AND status IN ('pending', 'processing');
