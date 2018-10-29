ALTER TABLE "bottle" DROP CONSTRAINT IF EXISTS "bottle_fk0";

ALTER TABLE "bottle" DROP CONSTRAINT IF EXISTS "bottle_fk1";

ALTER TABLE "bottle" DROP CONSTRAINT IF EXISTS "bottle_fk2";

ALTER TABLE "guild_bottle" DROP CONSTRAINT IF EXISTS "guild_bottle_fk0";

ALTER TABLE "guild_bottle" DROP CONSTRAINT IF EXISTS "guild_bottle_fk1";

ALTER TABLE "guild_contribution" DROP CONSTRAINT IF EXISTS "guild_contribution_fk0";

ALTER TABLE "guild_contribution" DROP CONSTRAINT IF EXISTS "guild_contribution_fk1";

ALTER TABLE "report" DROP CONSTRAINT IF EXISTS "report_fk0";

ALTER TABLE "report" DROP CONSTRAINT IF EXISTS "report_fk1";

ALTER TABLE "ban" DROP CONSTRAINT IF EXISTS "ban_fk0";

ALTER TABLE "ban" DROP CONSTRAINT IF EXISTS "ban_fk1";

DROP TABLE IF EXISTS "guild";

DROP TABLE IF EXISTS "bottle";

DROP VIEW IF EXISTS "user_rank";

DROP TABLE IF EXISTS "user";

DROP TABLE IF EXISTS "guild_bottle";

DROP VIEW IF EXISTS "guild_rank";

DROP TABLE IF EXISTS "guild_contribution";

DROP TABLE IF EXISTS "report";

DROP TABLE IF EXISTS "ban";

DROP FUNCTION public.estimate_rows(text);