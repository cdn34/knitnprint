CREATE TABLE staff_login_attempts (
    email citext PRIMARY KEY,
    failed_count integer NOT NULL DEFAULT 0,
    window_started_at timestamptz NOT NULL DEFAULT now(),
    locked_until timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT staff_login_attempts_count_positive CHECK (failed_count >= 0)
);

CREATE INDEX staff_login_attempts_cleanup
    ON staff_login_attempts (updated_at);
