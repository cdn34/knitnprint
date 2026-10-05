ALTER TABLE auth_login_rate_limits
    DROP CONSTRAINT auth_login_rate_limits_scope_valid,
    ADD CONSTRAINT auth_login_rate_limits_scope_valid
        CHECK (auth_scope IN ('staff', 'customer', 'account_action', 'feedback'));

CREATE INDEX product_feedback_admin_page_idx
    ON product_feedback (status, created_at DESC, id DESC);

CREATE TABLE feedback_settings (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    daily_submission_limit integer NOT NULL DEFAULT 20 CHECK (daily_submission_limit BETWEEN 1 AND 10000),
    hourly_ip_limit integer NOT NULL DEFAULT 2 CHECK (hourly_ip_limit BETWEEN 1 AND 1000)
);
INSERT INTO feedback_settings (singleton) VALUES (true);
