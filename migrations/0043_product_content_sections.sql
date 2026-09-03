ALTER TABLE products
    ADD COLUMN additional_information text NOT NULL DEFAULT '',
    ADD COLUMN care_instructions text NOT NULL DEFAULT '';

ALTER TABLE products
    ADD CONSTRAINT products_additional_information_length
        CHECK (length(additional_information) <= 20000),
    ADD CONSTRAINT products_care_instructions_length
        CHECK (length(care_instructions) <= 20000);
