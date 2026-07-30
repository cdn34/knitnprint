CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE staff_users (
    id uuid PRIMARY KEY,
    email citext NOT NULL UNIQUE,
    password_hash text NOT NULL,
    role text NOT NULL,
    display_name text NOT NULL,
    disabled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT staff_users_email_not_blank CHECK (length(btrim(email::text)) > 0),
    CONSTRAINT staff_users_display_name_not_blank CHECK (length(btrim(display_name)) > 0),
    CONSTRAINT staff_users_role_valid CHECK (role IN ('owner', 'staff'))
);

CREATE TABLE capabilities (
    name text PRIMARY KEY,
    description text NOT NULL,
    CONSTRAINT capabilities_name_not_blank CHECK (length(btrim(name)) > 0)
);

CREATE TABLE staff_capabilities (
    staff_user_id uuid NOT NULL REFERENCES staff_users(id) ON DELETE CASCADE,
    capability_name text NOT NULL REFERENCES capabilities(name) ON DELETE RESTRICT,
    granted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (staff_user_id, capability_name)
);

CREATE TABLE staff_sessions (
    id uuid PRIMARY KEY,
    staff_user_id uuid NOT NULL REFERENCES staff_users(id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    revoked_at timestamptz,
    CONSTRAINT staff_sessions_expiry_after_creation CHECK (expires_at > created_at)
);

CREATE INDEX staff_sessions_active_lookup
    ON staff_sessions (token_hash, expires_at)
    WHERE revoked_at IS NULL;

CREATE TABLE audit_log (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    action text NOT NULL,
    entity_type text NOT NULL,
    entity_id text,
    reason text,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT audit_log_action_not_blank CHECK (length(btrim(action)) > 0),
    CONSTRAINT audit_log_entity_type_not_blank CHECK (length(btrim(entity_type)) > 0)
);

CREATE OR REPLACE FUNCTION prevent_audit_log_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is immutable';
END;
$$;

CREATE TRIGGER audit_log_immutable
BEFORE UPDATE OR DELETE ON audit_log
FOR EACH ROW EXECUTE FUNCTION prevent_audit_log_mutation();

INSERT INTO capabilities (name, description) VALUES
    ('catalog.read', 'View catalog data'),
    ('catalog.write', 'Create and change catalog data'),
    ('inventory.adjust', 'Adjust inventory with a reason'),
    ('orders.read', 'View customer orders'),
    ('orders.fulfill', 'Fulfill paid orders'),
    ('orders.refund', 'Issue eligible refunds'),
    ('customers.read', 'View customer records'),
    ('media.upload', 'Upload product media'),
    ('media.review', 'Review uploaded media'),
    ('staff.manage', 'Create, change, and disable staff'),
    ('settings.manage', 'Manage store settings');

