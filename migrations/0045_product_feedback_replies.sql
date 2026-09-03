ALTER TABLE product_feedback
    ADD COLUMN store_reply text CHECK (
        store_reply IS NULL OR char_length(store_reply) BETWEEN 2 AND 1200
    ),
    ADD COLUMN replied_at timestamptz,
    ADD COLUMN replied_by uuid REFERENCES staff_users(id) ON DELETE SET NULL;
