ALTER TABLE inventory_items
DROP CONSTRAINT inventory_available_non_negative;

ALTER TABLE inventory_movements
DROP CONSTRAINT inventory_movement_result_non_negative;

COMMENT ON COLUMN inventory_items.available_quantity IS
'Real stock available to administrators. Negative values represent units that must be replenished.';
