-- Checkout language must stay fixed while delayed notifications are delivered.
CREATE FUNCTION prevent_order_locale_changes() RETURNS trigger AS $$
BEGIN
    IF NEW.email_locale IS DISTINCT FROM OLD.email_locale THEN
        RAISE EXCEPTION 'order language snapshot is immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER orders_locale_immutable BEFORE UPDATE OF email_locale ON orders
FOR EACH ROW EXECUTE FUNCTION prevent_order_locale_changes();
