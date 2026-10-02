CREATE TABLE product_feedback (
    id uuid PRIMARY KEY,
    product_id uuid REFERENCES products(id) ON DELETE CASCADE,
    display_name text NOT NULL CHECK (char_length(display_name) BETWEEN 2 AND 100),
    rating smallint NOT NULL CHECK (rating BETWEEN 1 AND 5),
    comment text NOT NULL CHECK (char_length(comment) BETWEEN 10 AND 1200),
    status text NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    created_at timestamptz NOT NULL DEFAULT now(),
    moderated_at timestamptz,
    moderated_by uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    store_reply text CHECK (store_reply IS NULL OR char_length(store_reply) BETWEEN 2 AND 1200),
    replied_at timestamptz,
    replied_by uuid REFERENCES staff_users(id) ON DELETE SET NULL
);

CREATE INDEX product_feedback_public_idx
    ON product_feedback (product_id, created_at DESC)
    WHERE status = 'approved';

CREATE INDEX product_feedback_homepage_idx
    ON product_feedback (created_at DESC, id DESC)
    WHERE status = 'approved';

CREATE INDEX product_feedback_moderation_idx
    ON product_feedback (status, created_at DESC);
