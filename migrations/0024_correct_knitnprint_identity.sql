UPDATE store_settings
SET store_name = 'KnitNPrint',
    updated_at = now()
WHERE store_name = 'KnitPrint';

UPDATE store_settings
SET support_email = 'hello@knitnprint.local',
    updated_at = now()
WHERE support_email = 'hello@knitprint.local';
