INSERT INTO capabilities (name, description)
VALUES ('discounts.manage', 'Create and manage discount codes');

CREATE TABLE discounts (
    id uuid PRIMARY KEY,
    code text NOT NULL UNIQUE,
    kind text NOT NULL,
    fixed_amount_minor bigint,
    percentage_basis_points integer,
    currency char(3) NOT NULL,
    minimum_order_minor bigint NOT NULL DEFAULT 0,
    starts_at timestamptz,
    ends_at timestamptz,
    usage_limit bigint,
    per_customer_limit bigint,
    status text NOT NULL DEFAULT 'active',
    created_by_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    disabled_by_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    disabled_reason text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT discounts_code_valid CHECK (code ~ '^[A-Z0-9][A-Z0-9_-]{2,31}$'),
    CONSTRAINT discounts_kind_valid CHECK (kind IN ('fixed', 'percentage')),
    CONSTRAINT discounts_value_consistent CHECK (
        (kind = 'fixed' AND fixed_amount_minor > 0 AND percentage_basis_points IS NULL)
        OR (kind = 'percentage' AND fixed_amount_minor IS NULL AND percentage_basis_points BETWEEN 1 AND 10000)
    ),
    CONSTRAINT discounts_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT discounts_minimum_non_negative CHECK (minimum_order_minor >= 0),
    CONSTRAINT discounts_dates_ordered CHECK (starts_at IS NULL OR ends_at IS NULL OR starts_at < ends_at),
    CONSTRAINT discounts_usage_limit_positive CHECK (usage_limit IS NULL OR usage_limit > 0),
    CONSTRAINT discounts_customer_limit_positive CHECK (per_customer_limit IS NULL OR per_customer_limit > 0),
    CONSTRAINT discounts_status_valid CHECK (status IN ('active', 'disabled')),
    CONSTRAINT discounts_disabled_consistent CHECK (
        (status = 'active' AND disabled_by_staff_user_id IS NULL AND disabled_reason IS NULL)
        OR (status = 'disabled' AND disabled_reason IS NOT NULL AND length(btrim(disabled_reason)) BETWEEN 3 AND 500)
    )
);

CREATE INDEX discounts_availability ON discounts (status, starts_at, ends_at);

ALTER TABLE carts ADD COLUMN discount_id uuid REFERENCES discounts(id) ON DELETE SET NULL;
CREATE INDEX carts_discount ON carts (discount_id) WHERE discount_id IS NOT NULL;

CREATE TABLE order_discounts (
    order_id uuid PRIMARY KEY REFERENCES orders(id) ON DELETE RESTRICT,
    discount_id uuid REFERENCES discounts(id) ON DELETE SET NULL,
    code text NOT NULL,
    kind text NOT NULL,
    fixed_amount_minor bigint,
    percentage_basis_points integer,
    amount_minor bigint NOT NULL,
    currency char(3) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT order_discounts_code_not_blank CHECK (length(btrim(code)) BETWEEN 3 AND 32),
    CONSTRAINT order_discounts_kind_valid CHECK (kind IN ('fixed', 'percentage')),
    CONSTRAINT order_discounts_value_consistent CHECK (
        (kind = 'fixed' AND fixed_amount_minor > 0 AND percentage_basis_points IS NULL)
        OR (kind = 'percentage' AND fixed_amount_minor IS NULL AND percentage_basis_points BETWEEN 1 AND 10000)
    ),
    CONSTRAINT order_discounts_amount_positive CHECK (amount_minor > 0),
    CONSTRAINT order_discounts_currency_uppercase CHECK (currency ~ '^[A-Z]{3}$')
);

CREATE TABLE discount_usages (
    id uuid PRIMARY KEY,
    discount_id uuid NOT NULL REFERENCES discounts(id) ON DELETE RESTRICT,
    order_id uuid NOT NULL UNIQUE REFERENCES orders(id) ON DELETE RESTRICT,
    customer_id uuid REFERENCES customers(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX discount_usages_global ON discount_usages (discount_id, created_at, id);
CREATE INDEX discount_usages_customer ON discount_usages (discount_id, customer_id)
    WHERE customer_id IS NOT NULL;

CREATE TRIGGER order_discounts_immutable
BEFORE UPDATE OR DELETE ON order_discounts
FOR EACH ROW EXECUTE FUNCTION prevent_order_snapshot_changes();

COMMENT ON TABLE discounts IS 'Server-owned fixed and percentage promotion rules managed by authorized staff.';
COMMENT ON TABLE order_discounts IS 'Immutable discount rule and amount snapshot retained with an order.';
COMMENT ON TABLE discount_usages IS 'One durable usage record per discounted order for concurrency-safe limits.';
