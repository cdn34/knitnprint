CREATE TABLE b2b_requests (
    id uuid PRIMARY KEY,
    reference_number bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    request_key uuid NOT NULL UNIQUE,
    request_hash bytea NOT NULL CHECK (octet_length(request_hash) = 32),
    company text NOT NULL CHECK (length(btrim(company)) BETWEEN 1 AND 200),
    contact_name text NOT NULL CHECK (length(btrim(contact_name)) BETWEEN 1 AND 150),
    email text NOT NULL CHECK (length(email) BETWEEN 3 AND 254),
    phone text NOT NULL CHECK (length(btrim(phone)) BETWEEN 3 AND 40),
    product_type text NOT NULL CHECK (product_type IN ('clothing','bottles','backpacks','complete-kit','other')),
    quantity integer NOT NULL CHECK (quantity BETWEEN 1 AND 1000000),
    notes text NOT NULL DEFAULT '' CHECK (length(notes) <= 5000),
    status text NOT NULL DEFAULT 'new' CHECK (status IN ('new','in_progress','completed')),
    attachment_name text NOT NULL CHECK (length(attachment_name) BETWEEN 1 AND 200),
    attachment_type text NOT NULL,
    attachment_bytes bytea NOT NULL CHECK (octet_length(attachment_bytes) BETWEEN 1 AND 10485760),
    attachment_sha256 bytea NOT NULL CHECK (octet_length(attachment_sha256) = 32),
    preview_bytes bytea,
    preview_type text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((preview_bytes IS NULL) = (preview_type IS NULL))
);
CREATE INDEX b2b_requests_inbox ON b2b_requests (created_at DESC, id DESC);

CREATE TABLE b2b_notification_jobs (
    request_id uuid PRIMARY KEY REFERENCES b2b_requests(id) ON DELETE RESTRICT,
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','processing','sent','failed')),
    attempt_count integer NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at timestamptz NOT NULL DEFAULT now(),
    locked_at timestamptz,
    sent_at timestamptz,
    last_error text,
    CHECK ((status = 'sent') = (sent_at IS NOT NULL))
);
CREATE INDEX b2b_notification_jobs_due ON b2b_notification_jobs (next_attempt_at)
    WHERE status IN ('pending','processing');
COMMENT ON TABLE b2b_requests IS 'Private business enquiries. Original attachments and image previews are persisted as PostgreSQL bytea and included in database backups.';
