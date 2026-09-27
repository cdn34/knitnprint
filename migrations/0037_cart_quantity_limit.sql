ALTER TABLE cart_lines
DROP CONSTRAINT cart_lines_quantity_valid,
ADD CONSTRAINT cart_lines_quantity_valid CHECK (quantity BETWEEN 1 AND 100);

ALTER TABLE order_lines
DROP CONSTRAINT order_lines_quantity_positive,
ADD CONSTRAINT order_lines_quantity_positive CHECK (quantity BETWEEN 1 AND 100);

ALTER TABLE fulfillment_lines
DROP CONSTRAINT fulfillment_lines_quantity_positive,
ADD CONSTRAINT fulfillment_lines_quantity_positive CHECK (quantity BETWEEN 1 AND 100);

ALTER TABLE order_refund_lines
DROP CONSTRAINT order_refund_lines_quantity_positive,
ADD CONSTRAINT order_refund_lines_quantity_positive CHECK (quantity BETWEEN 1 AND 100);
