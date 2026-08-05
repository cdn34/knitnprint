CREATE TABLE customer_account_tokens (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customer_accounts(customer_id) ON DELETE CASCADE,
    token_kind text NOT NULL,
    token_hash bytea NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT customer_account_tokens_kind_valid
        CHECK (token_kind IN ('email_verification', 'password_reset')),
    CONSTRAINT customer_account_tokens_hash_length CHECK (octet_length(token_hash) = 32),
    CONSTRAINT customer_account_tokens_expiry_after_creation CHECK (expires_at > created_at),
    CONSTRAINT customer_account_tokens_use_after_creation
        CHECK (used_at IS NULL OR used_at >= created_at)
);

CREATE INDEX customer_account_tokens_active_customer
    ON customer_account_tokens (customer_id, token_kind, created_at DESC)
    WHERE used_at IS NULL;

CREATE INDEX customer_account_tokens_cleanup
    ON customer_account_tokens (expires_at)
    WHERE used_at IS NULL;

COMMENT ON TABLE customer_account_tokens IS
    'Single-use email verification and password-reset secrets stored only as SHA-256 hashes.';
