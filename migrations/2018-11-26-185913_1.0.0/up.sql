DELETE FROM bottle;
DELETE FROM guild_bottle;
ALTER TABLE guild_bottle RENAME TO received_bottle;

ALTER TABLE received_bottle ADD COLUMN channel bigint NOT NULL;
ALTER TABLE received_bottle DROP COLUMN guild;

ALTER TABLE bottle ADD COLUMN channel bigint NOT NULL;