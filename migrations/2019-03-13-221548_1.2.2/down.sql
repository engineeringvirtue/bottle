ALTER TABLE report DROP COLUMN received_bottle;
ALTER TABLE report ADD COLUMN message bigint NOT NULL UNIQUE;
