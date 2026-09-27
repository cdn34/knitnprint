UPDATE shipping_package_profiles
SET name = regexp_replace(name, '^Embalagem existente ', 'Existing package '),
    updated_at = now()
WHERE name ~ '^Embalagem existente [0-9]+$';
