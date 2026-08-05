CREATE TABLE auth_login_rate_limits (
    auth_scope text NOT NULL,
    dimension text NOT NULL,
    key_hash bytea NOT NULL,
    event_count integer NOT NULL DEFAULT 0,
    window_started_at timestamptz NOT NULL DEFAULT now(),
    locked_until timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (auth_scope, dimension, key_hash),
    CONSTRAINT auth_login_rate_limits_scope_valid
        CHECK (auth_scope IN ('staff', 'customer')),
    CONSTRAINT auth_login_rate_limits_dimension_valid
        CHECK (dimension IN ('account', 'ip', 'global')),
    CONSTRAINT auth_login_rate_limits_hash_length CHECK (octet_length(key_hash) = 32),
    CONSTRAINT auth_login_rate_limits_count_positive CHECK (event_count >= 0)
);

CREATE INDEX auth_login_rate_limits_cleanup
    ON auth_login_rate_limits (auth_scope, updated_at);

COMMENT ON TABLE auth_login_rate_limits IS
    'Hashed account/IP and non-personal global login buckets; advisory locks serialize threshold decisions.';

DROP TABLE staff_login_attempts;
DROP TABLE customer_login_attempts;
