CREATE TABLE "guild" (
	"id" bigint NOT NULL,
	"bottle_channel" bigint UNIQUE,
	"admin_channel" bigint UNIQUE,
	CONSTRAINT guild_pk PRIMARY KEY ("id")
) WITH (
  OIDS=FALSE
);



CREATE TABLE "bottle" (
	"id" bigserial NOT NULL,
	"reply_to" bigint,
	"user" bigint NOT NULL,
	"message" bigint NOT NULL UNIQUE,
	"guild" bigint,
	"time_pushed" TIMESTAMP NOT NULL DEFAULT 'NOW()',
	"contents" TEXT NOT NULL,
	CONSTRAINT bottle_pk PRIMARY KEY ("id")
) WITH (
  OIDS=FALSE
);



CREATE TABLE "user" (
	"id" bigint NOT NULL,
	"subscribed" bool NOT NULL DEFAULT 'true',
	"token" TEXT DEFAULT 'NULL',
	"xp" integer NOT NULL DEFAULT '0',
	"admin" bool NOT NULL DEFAULT 'false',
	CONSTRAINT user_pk PRIMARY KEY ("id")
) WITH (
  OIDS=FALSE
);



CREATE TABLE "guild_bottle" (
	"id" bigserial NOT NULL,
	"bottle" bigserial NOT NULL UNIQUE,
	"guild" bigint NOT NULL UNIQUE,
	"message" bigint NOT NULL UNIQUE,
	"time_recieved" TIMESTAMP NOT NULL DEFAULT 'NOW()',
	CONSTRAINT guild_bottle_pk PRIMARY KEY ("id")
) WITH (
  OIDS=FALSE
);



CREATE TABLE "report" (
	"bottle" bigserial NOT NULL,
	"user" bigint NOT NULL,
	CONSTRAINT report_pk PRIMARY KEY ("bottle")
) WITH (
  OIDS=FALSE
);



CREATE TABLE "ban" (
	"report" bigserial NOT NULL,
	"user" bigint NOT NULL,
	CONSTRAINT ban_pk PRIMARY KEY ("user")
) WITH (
  OIDS=FALSE
);




ALTER TABLE "bottle" ADD CONSTRAINT "bottle_fk0" FOREIGN KEY ("reply_to") REFERENCES "bottle"("id");
ALTER TABLE "bottle" ADD CONSTRAINT "bottle_fk1" FOREIGN KEY ("user") REFERENCES "user"("id");
ALTER TABLE "bottle" ADD CONSTRAINT "bottle_fk2" FOREIGN KEY ("guild") REFERENCES "guild"("id");


ALTER TABLE "guild_bottle" ADD CONSTRAINT "guild_bottle_fk0" FOREIGN KEY ("bottle") REFERENCES "bottle"("id");
ALTER TABLE "guild_bottle" ADD CONSTRAINT "guild_bottle_fk1" FOREIGN KEY ("guild") REFERENCES "guild"("id");

ALTER TABLE "report" ADD CONSTRAINT "report_fk0" FOREIGN KEY ("bottle") REFERENCES "bottle"("id");
ALTER TABLE "report" ADD CONSTRAINT "report_fk1" FOREIGN KEY ("user") REFERENCES "user"("id");

ALTER TABLE "ban" ADD CONSTRAINT "ban_fk0" FOREIGN KEY ("report") REFERENCES "report"("bottle");
ALTER TABLE "ban" ADD CONSTRAINT "ban_fk1" FOREIGN KEY ("user") REFERENCES "user"("id");

-- generated by pgadmin
CREATE OR REPLACE FUNCTION public.estimate_rows(
	tablename text)
    RETURNS bigint
    LANGUAGE 'sql'

    COST 100
    VOLATILE
AS $BODY$

SELECT reltuples::bigint AS num FROM pg_class where relname=tablename;

$BODY$;