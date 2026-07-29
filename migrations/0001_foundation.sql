-- Internal application metadata only. Commercial tables belong to feature
-- migrations introduced by later vertical slices.
CREATE TABLE app_metadata (
    key text PRIMARY KEY,
    value jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT app_metadata_key_not_blank CHECK (length(btrim(key)) > 0)
);

COMMENT ON TABLE app_metadata IS
    'Internal schema and development seed metadata; not commercial settings.';

