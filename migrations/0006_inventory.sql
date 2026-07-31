CREATE TABLE inventory_items (
    variant_id uuid PRIMARY KEY REFERENCES product_variants(id) ON DELETE CASCADE,
    available_quantity bigint NOT NULL DEFAULT 0,
    reserved_quantity bigint NOT NULL DEFAULT 0,
    committed_quantity bigint NOT NULL DEFAULT 0,
    low_stock_threshold bigint NOT NULL DEFAULT 3,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT inventory_available_non_negative CHECK (available_quantity >= 0),
    CONSTRAINT inventory_reserved_non_negative CHECK (reserved_quantity >= 0),
    CONSTRAINT inventory_committed_non_negative CHECK (committed_quantity >= 0),
    CONSTRAINT inventory_threshold_non_negative CHECK (low_stock_threshold >= 0)
);

INSERT INTO inventory_items (variant_id)
SELECT id FROM product_variants;

CREATE FUNCTION create_variant_inventory_item() RETURNS trigger AS $$
BEGIN
    INSERT INTO inventory_items (variant_id) VALUES (NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER product_variant_inventory_item
AFTER INSERT ON product_variants
FOR EACH ROW EXECUTE FUNCTION create_variant_inventory_item();

CREATE TABLE inventory_movements (
    id uuid PRIMARY KEY,
    variant_id uuid NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    actor_staff_user_id uuid REFERENCES staff_users(id) ON DELETE SET NULL,
    movement_type text NOT NULL,
    quantity_delta bigint NOT NULL,
    resulting_available_quantity bigint NOT NULL,
    reason text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT inventory_movement_type_valid CHECK (movement_type IN ('adjustment', 'reservation', 'release', 'commitment')),
    CONSTRAINT inventory_movement_delta_non_zero CHECK (quantity_delta <> 0),
    CONSTRAINT inventory_movement_result_non_negative CHECK (resulting_available_quantity >= 0),
    CONSTRAINT inventory_movement_reason_not_blank CHECK (length(btrim(reason)) >= 3)
);

CREATE INDEX inventory_movements_variant_history
    ON inventory_movements (variant_id, created_at DESC, id DESC);

CREATE INDEX inventory_items_low_stock
    ON inventory_items (available_quantity, low_stock_threshold)
    WHERE available_quantity <= low_stock_threshold;

CREATE FUNCTION prevent_inventory_movement_changes() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'inventory movements are immutable';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER inventory_movements_immutable
BEFORE UPDATE OR DELETE ON inventory_movements
FOR EACH ROW EXECUTE FUNCTION prevent_inventory_movement_changes();
