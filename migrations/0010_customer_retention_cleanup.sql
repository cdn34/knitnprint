CREATE INDEX customers_retention_cleanup
    ON customers (retention_expires_at, id)
    WHERE anonymized_at IS NULL;

COMMENT ON COLUMN customers.anonymized_at IS
    'Marks a retained commercial identity whose personal data, addresses, credentials, and sessions were irreversibly removed.';
