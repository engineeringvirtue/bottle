DELETE FROM report;

ALTER TABLE report DROP COLUMN message;
ALTER TABLE report ADD COLUMN received_bottle bigserial UNIQUE REFERENCES received_bottle("id");
