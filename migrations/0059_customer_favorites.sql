CREATE TABLE customer_favorites (
    customer_id uuid NOT NULL REFERENCES customer_accounts(customer_id) ON DELETE CASCADE,
    product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (customer_id, product_id)
);

CREATE INDEX customer_favorites_recent
    ON customer_favorites (customer_id, created_at DESC, product_id DESC);
