CREATE OR REPLACE FUNCTION prevent_inventory_movement_changes() RETURNS trigger AS $$
BEGIN
    -- Preserve direct immutability while allowing movements to follow a product
    -- that is deliberately removed through its cascading foreign keys.
    IF TG_OP = 'DELETE' AND pg_trigger_depth() > 1 THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'inventory movements are immutable';
END;
$$ LANGUAGE plpgsql;
