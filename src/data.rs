use model::*;

use diesel::prelude::*;
use schema::*;
use diesel::*;
use diesel::dsl::sql;
use uuid::Uuid;

type Res<A> = Result<A, result::Error>;

pub mod functions {
    use diesel::sql_types::*;

    no_arg_sql_function!(random, (), "Represents the postgresql random() function");
    sql_function!(estimate_rows, Estimate, (tablename: Text) -> Int8);
}

use self::functions::*;

impl User {
    pub fn get(uid: UserId, conn:&Conn) -> Self {
        user::table.find(uid).first(conn).unwrap_or_else(|_| User::new(uid))
    }

    pub fn update(&self, conn:&Conn) -> Res<usize> {
        insert_into(user::table).values(self).on_conflict(user::id).do_update().set(self).execute(conn)
    }

    pub fn get_last_bottles(&self, limit:i64, conn:&Conn) -> Res<Vec<Bottle>> {
        Bottle::belonging_to(self).filter(bottle::guild.is_not_null()).filter(bottle::reply_to.is_null()).order(bottle::time_pushed.desc()).limit(limit).load(conn)
    }

    pub fn get_bottle(&self, conn:&Conn) -> Res<Bottle> {
        Bottle::belonging_to(self).order(bottle::time_pushed.desc()).limit(1).first(conn)
    }

    pub fn get_num_bottles(&self, conn:&Conn) -> Res<i64> {
        Bottle::belonging_to(self).select(dsl::count_star()).first(conn)
    }

    pub fn get_ranking(&self, conn:&Conn) -> Res<i64> {
        user_rank::table.find(self.id).select(user_rank::rank).first(conn)
    }

    pub fn get_banned(&self, conn:&Conn) -> Res<bool> {
        select(dsl::exists(ban::table.find(self.id))).get_result(conn)
    }

    pub fn from_session(ses:Uuid, conn:&Conn) -> Res<Self> {
        user::table.filter(user::session.eq(ses)).first(conn)
    }

    pub fn get_contributions(&self, limit:i64, conn:&Conn) -> Res<Vec<GuildContribution>> {
        guild_contribution::table.filter(guild_contribution::user.eq(self.id)).order(guild_contribution::xp.desc()).limit(limit).load(conn)
    }
}

impl Guild {
    pub fn get(gid: GuildId, conn:&Conn) -> Self {
        guild::table.find(gid).first(conn).unwrap_or_else(|_| Guild::new(gid))
    }

    pub fn update(&self, conn:&Conn) -> Res<usize> {
        insert_into(guild::table).values(self).on_conflict(guild::id).do_update().set(self).execute(conn)
    }

    pub fn get_last_bottle(&self, conn:&Conn) -> Res<GuildBottle> {
        GuildBottle::belonging_to(self).order(guild_bottle::time_recieved.desc()).first(conn)
    }

    pub fn get_contributions(&self, limit:i64, conn:&Conn) -> Res<Vec<GuildContribution>> {
        guild_contribution::table.filter(guild_contribution::guild.eq(self.id)).order(guild_contribution::xp.desc()).limit(limit).load(conn)
    }

    pub fn get_xp(&self, conn:&Conn) -> Res<Option<i64>> {
        guild_contribution::table.filter(guild_contribution::guild.eq(self.id)).select(dsl::sum(guild_contribution::xp)).first(conn)
    }

    pub fn get_ranking(&self, conn:&Conn) -> Res<i64> {
        guild_rank::table.find(self.id).select(guild_rank::rank).first(conn)
    }

    pub fn get_num_bottles(&self, conn:&Conn) -> Res<i64> {
        GuildBottle::belonging_to(self).select(dsl::count_star()).first(conn)
    }

    pub fn del(gid: GuildId, conn:&Conn) -> Res<usize> {
        delete(guild::table).filter(guild::id.eq(gid)).execute(conn)
    }
}

impl MakeBottle {
    pub fn make(&self, conn:&Conn) -> Res<Bottle> {
        insert_into(bottle::table).values(self).get_result(conn)
    }
}

impl Bottle {
    pub fn get(id:BottleId, conn:&Conn) -> Res<Self> {
        bottle::table.find(id).get_result(conn)
    }

    pub fn del(id:BottleId, conn:&Conn) -> Res<usize> {
        delete(bottle::table).filter(bottle::id.eq(id)).execute(conn)
    }

    pub fn get_reply_list(&self, conn:&Conn) -> Res<Vec<Self>> {
        let mut bottles: Vec<Bottle> = Vec::new();

        while bottles.len() < 25 {
            match bottles.last().unwrap_or(self).reply_to {
                Some(x) => {
                    bottles.push(Bottle::get(x, conn)?);
                },
                None => break
            }
        }

        bottles.insert(0, self.clone());
        Ok(bottles)
    }
}

impl MakeGuildBottle {
    pub fn make(&self, conn:&Conn) -> Res<GuildBottle> {
        insert_into(guild_bottle::table).values(self).get_result(conn)
    }
}

impl GuildBottle {
    pub fn get(buid:GuildBottleId, conn:&Conn) -> Res<Self> {
        guild_bottle::table.find(buid).get_result(conn)
    }

    pub fn get_from_bottle(bid: BottleId, conn:&Conn) -> Res<Vec<Self>> {
        guild_bottle::table.filter(guild_bottle::bottle.eq(bid)).load(conn)
    }

    pub fn get_from_message(mid:i64, conn:&Conn) -> Res<Self> {
        guild_bottle::table.filter(guild_bottle::message.eq(mid)).get_result(conn)
    }

    pub fn del(&self, conn:&Conn) -> Res<usize> {
        delete(guild_bottle::table.find(self.id)).execute(conn)
    }
}

impl GuildContribution {
    pub fn get(id: GuildContributionId, conn:&Conn) -> Self {
        guild_contribution::table.find(id).first(conn).unwrap_or_else(|_| GuildContribution {guild: id.0, user: id.1, xp: 0})
    }

    pub fn update(&self, conn:&Conn) -> Res<Self> {
        insert_into(guild_contribution::table).values(self)
            .on_conflict((guild_contribution::guild, guild_contribution::user)).do_update().set(self).get_result(conn)
    }
}

impl Report {
    pub fn make(&self, conn:&Conn) -> Res<Self> {
        insert_into(report::table).values(self).get_result(conn)
    }

    pub fn get_from_message(mid:i64, conn:&Conn) -> Res<Self> {
        report::table.filter(report::message.eq(mid)).first(conn)
    }

    pub fn del(&self, conn:&Conn) -> Res<usize> {
        delete(report::table.find(self.bottle)).execute(conn)
    }
}

impl Ban {
    pub fn make(&self, conn:&Conn) -> Res<Self> {
        insert_into(ban::table).values(self).get_result(conn)
    }

    pub fn del(&self, conn:&Conn) -> Res<usize> {
        delete(ban::table.find(self.user)).execute(conn)
    }
}

pub fn get_bottle_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("bottle".to_owned())).get_result(conn)
}

pub fn get_user_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("user".to_owned())).get_result(conn)
}

pub fn get_guild_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("guild".to_owned())).get_result(conn)
}