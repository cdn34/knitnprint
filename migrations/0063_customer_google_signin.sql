ALTER TABLE customer_accounts ALTER COLUMN password_hash DROP NOT NULL;
ALTER TABLE customer_accounts ADD COLUMN google_auto_link_disabled boolean NOT NULL DEFAULT false;
CREATE TABLE customer_google_identities (
    customer_id uuid PRIMARY KEY REFERENCES customer_accounts(customer_id) ON DELETE CASCADE,
    subject text NOT NULL UNIQUE CHECK (length(subject) BETWEEN 1 AND 255),
    email citext NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE customer_sessions ADD COLUMN reauthenticated_at timestamptz;
ALTER TABLE customer_sessions ADD COLUMN reauthenticated_method text CHECK (reauthenticated_method IN ('password', 'google'));
CREATE TABLE customer_google_challenges (
    id uuid PRIMARY KEY,
    browser_hash bytea NOT NULL,
    nonce_hash bytea NOT NULL,
    purpose text NOT NULL CHECK (purpose IN ('sign_in', 'link', 'reauthenticate')),
    customer_id uuid REFERENCES customer_accounts(customer_id) ON DELETE CASCADE,
    session_hash bytea,
    subject text,
    email citext,
    first_name text NOT NULL DEFAULT '',
    last_name text NOT NULL DEFAULT '',
    trusted_email boolean NOT NULL DEFAULT false,
    confirmation_hash bytea,
    confirmed_at timestamptz,
    expires_at timestamptz NOT NULL DEFAULT now() + interval '10 minutes',
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX customer_google_challenges_expiry ON customer_google_challenges(expires_at);
