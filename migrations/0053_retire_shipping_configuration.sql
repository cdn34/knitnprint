-- The application retires the previous delivery-pricing model, but the previous
-- API revision still needs its tables and columns during migration-first
-- deployments. Retain that schema until a later release can safely remove it.
-- Existing order snapshots and delivery addresses remain untouched.
SELECT 1;
