CREATE TABLE "guild" (
	"guildid" bigint NOT NULL,
	"admin_channel" bigint NOT NULL,
	CONSTRAINT guild_pk PRIMARY KEY ("guildid")
) WITH (
  OIDS=FALSE
);

CREATE TABLE "user" (
	"userid" bigint NOT NULL,
	"subscribed" bool NOT NULL DEFAULT 'true',
	"token" TEXT DEFAULT 'NULL',
	"xp" bigint NOT NULL DEFAULT '0',
	CONSTRAINT user_pk PRIMARY KEY ("userid")
) WITH (
  OIDS=FALSE
);

CREATE TABLE "bottle" (
	"bottleid" bigserial NOT NULL,
	"user" bigint NOT NULL,

	"reply_to" bigserial,

	"messageid" bigint NOT NULL,
	"time_pushed" TIMESTAMP NOT NULL,
	"message" TEXT NOT NULL,
	CONSTRAINT bottle_pk PRIMARY KEY ("bottleid"),
    CONSTRAINT bottle_unique UNIQUE ("messageid")
) WITH (
  OIDS=FALSE
);

CREATE TABLE "bottle_user" (
	"bottle" bigserial NOT NULL,
	"user" bigint NOT NULL,
	"messageid" bigint NOT NULL,
	"time_recieved" TIMESTAMP NOT NULL,
	CONSTRAINT bottle_user_pk PRIMARY KEY ("bottle","user"),
	CONSTRAINT bottle_user_unique UNIQUE ("messageid")
) WITH (
  OIDS=FALSE
);

CREATE TABLE "report" (
	"reportid" bigserial NOT NULL,
	"bottle" bigserial NOT NULL,
	
	"guild" bigint NOT NULL,
	"messageid" bigint NOT NULL,
	"user" bigint NOT NULL,
	CONSTRAINT report_pk PRIMARY KEY ("reportid"),
	CONSTRAINT report_unique UNIQUE ("messageid")
) WITH (
  OIDS=FALSE
);



ALTER TABLE "bottle" ADD CONSTRAINT "bottle_fk0" FOREIGN KEY ("user") REFERENCES "user"("userid");
ALTER TABLE "bottle" ADD CONSTRAINT "bottle_fk1" FOREIGN KEY ("reply_to") REFERENCES "bottle"("bottleid");

ALTER TABLE "bottle_user" ADD CONSTRAINT "bottle_user_fk0" FOREIGN KEY ("bottle") REFERENCES "bottle"("bottleid");
ALTER TABLE "bottle_user" ADD CONSTRAINT "bottle_user_fk1" FOREIGN KEY ("user") REFERENCES "user"("userid");

ALTER TABLE "report" ADD CONSTRAINT "report_fk0" FOREIGN KEY ("bottle") REFERENCES "bottle"("bottleid");
ALTER TABLE "report" ADD CONSTRAINT "report_fk1" FOREIGN KEY ("guild") REFERENCES "guild"("guildid");
ALTER TABLE "report" ADD CONSTRAINT "report_fk2" FOREIGN KEY ("user") REFERENCES "user"("userid");
