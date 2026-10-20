ALTER TABLE customer_account_tokens
    DROP CONSTRAINT customer_account_tokens_kind_valid,
    ADD CONSTRAINT customer_account_tokens_kind_valid
        CHECK (token_kind IN ('email_verification', 'password_reset', 'email_change')),
    ADD COLUMN target_email citext,
    ADD CONSTRAINT customer_account_tokens_email_change_target
        CHECK ((token_kind = 'email_change') = (target_email IS NOT NULL));

COMMENT ON COLUMN customer_account_tokens.target_email IS
    'New address proposed by the customer; their current address stays active until the token is confirmed.';
