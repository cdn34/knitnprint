CREATE UNIQUE INDEX customers_registered_email_unique
    ON customers (email)
    WHERE customer_type = 'registered' AND anonymized_at IS NULL;

CREATE TABLE customer_accounts (
    customer_id uuid PRIMARY KEY REFERENCES customers(id) ON DELETE CASCADE,
    password_hash text NOT NULL,
    email_verified_at timestamptz,
    disabled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT customer_accounts_password_hash_not_blank CHECK (length(btrim(password_hash)) > 0)
);

CREATE TABLE customer_sessions (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customer_accounts(customer_id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    CONSTRAINT customer_sessions_token_hash_length CHECK (octet_length(token_hash) = 32),
    CONSTRAINT customer_sessions_expiry_after_creation CHECK (expires_at > created_at)
);

CREATE INDEX customer_sessions_active_lookup
    ON customer_sessions (token_hash, expires_at)
    WHERE revoked_at IS NULL;

CREATE TABLE customer_login_attempts (
    email citext PRIMARY KEY,
    failed_count integer NOT NULL DEFAULT 0,
    window_started_at timestamptz NOT NULL DEFAULT now(),
    locked_until timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT customer_login_attempts_count_positive CHECK (failed_count >= 0)
);

CREATE INDEX customer_login_attempts_cleanup
    ON customer_login_attempts (updated_at);

ALTER TABLE audit_log
    ADD COLUMN actor_customer_id uuid REFERENCES customers(id) ON DELETE SET NULL,
    ADD CONSTRAINT audit_log_single_actor CHECK (
        actor_staff_user_id IS NULL OR actor_customer_id IS NULL
    );

COMMENT ON TABLE customer_accounts IS
    'Optional storefront identities; staff authentication remains in staff_users.';
COMMENT ON TABLE customer_sessions IS
    'Opaque storefront sessions stored only as SHA-256 token hashes.';
