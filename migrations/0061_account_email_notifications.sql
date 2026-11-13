ALTER TABLE notification_jobs
    ADD COLUMN account_token_id uuid REFERENCES customer_account_tokens(id) ON DELETE SET NULL,
    ADD CONSTRAINT notification_jobs_account_token_consistent CHECK (
        account_token_id IS NULL OR (
            kind IN ('email_verification', 'password_reset', 'email_change')
            AND order_id IS NULL AND fulfillment_id IS NULL
        )
    );

CREATE INDEX notification_jobs_account_token ON notification_jobs(account_token_id)
    WHERE account_token_id IS NOT NULL;

ALTER TABLE notification_attempts DROP CONSTRAINT notification_attempts_outcome_valid,
    ADD CONSTRAINT notification_attempts_outcome_valid CHECK (outcome IN ('sent', 'failed', 'suppressed'));

-- Authentication links are regenerated in memory at delivery, never stored in
-- the outbox. Purging an account token also removes its outbox recipient data.
CREATE FUNCTION redact_account_notification_recipient() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE notification_jobs
    SET recipient_email = 'anonymized@knitnprint.invalid',
        status = CASE WHEN status = 'sent' THEN 'sent' ELSE 'suppressed' END,
        locked_at = NULL,
        last_error = CASE WHEN status = 'sent' THEN NULL ELSE 'Account request is no longer available.' END,
        updated_at = now()
    WHERE account_token_id = OLD.id;
    RETURN OLD;
END;
$$;

CREATE TRIGGER account_notification_recipient_retention
BEFORE DELETE ON customer_account_tokens
FOR EACH ROW EXECUTE FUNCTION redact_account_notification_recipient();
