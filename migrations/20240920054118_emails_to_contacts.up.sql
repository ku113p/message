ALTER TABLE message
    DROP COLUMN email,
    ADD COLUMN contacts jsonb NOT NULL DEFAULT '{}';
