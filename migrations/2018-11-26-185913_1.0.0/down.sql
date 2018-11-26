ALTER TABLE received_bottle DROP COLUMN IF EXISTS channel CASCADE;
ALTER TABLE received_bottle ADD COLUMN guild bigint NOT NULL REFERENCES guild("id");
ALTER TABLE received_bottle RENAME TO guild_bottle;

ALTER TABLE bottle DROP COLUMN IF EXISTS channel CASCADE;