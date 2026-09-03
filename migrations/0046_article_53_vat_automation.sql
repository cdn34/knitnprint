ALTER TABLE store_settings
ADD COLUMN vat_automation_enabled boolean NOT NULL DEFAULT true,
ADD COLUMN vat_activated_at timestamptz,
ADD COLUMN vat_activation_reason text;

ALTER TABLE store_settings
ADD CONSTRAINT store_settings_vat_activation_consistent CHECK (
    (vat_activated_at IS NULL AND vat_activation_reason IS NULL)
    OR (vat_activated_at IS NOT NULL AND length(btrim(vat_activation_reason)) BETWEEN 3 AND 200)
);

INSERT INTO tax_rules (id, name, country_codes, rate_basis_points, priority, active)
SELECT
    '00000000-0000-7000-8000-000000000046',
    'Portugal standard VAT',
    ARRAY['PT']::text[],
    2300,
    COALESCE((SELECT max(priority) + 1 FROM tax_rules), 0),
    true
WHERE NOT EXISTS (
    SELECT 1
    FROM tax_rules
    WHERE active
      AND 'PT' = ANY(country_codes)
);

COMMENT ON COLUMN store_settings.vat_automation_enabled IS
'Automatically applies the Portuguese Article 53 turnover transitions.';
COMMENT ON COLUMN store_settings.vat_activated_at IS
'Latched timestamp after automatic entry into the standard VAT regime; never cleared automatically.';
COMMENT ON COLUMN store_settings.vat_activation_reason IS
'Auditable reason for the automatic VAT regime transition.';
