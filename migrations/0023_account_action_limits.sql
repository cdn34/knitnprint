ALTER TABLE auth_login_rate_limits
    DROP CONSTRAINT auth_login_rate_limits_scope_valid,
    ADD CONSTRAINT auth_login_rate_limits_scope_valid
        CHECK (auth_scope IN ('staff', 'customer', 'account_action'));

COMMENT ON TABLE auth_login_rate_limits IS
    'Hashed account/IP/global buckets for staff and customer login plus sensitive account-email actions.';
