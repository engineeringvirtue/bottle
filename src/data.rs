use model::*;

use diesel::prelude::*;
use schema::*;
use diesel::*;

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

    pub fn get_last_bottle(&self, conn:&Conn) -> Res<Bottle> {
        Bottle::belonging_to(self).order(bottle::time_pushed.desc()).first(conn)
    }

}
//
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

    pub fn delete(gid: GuildId, conn:&Conn) -> Res<usize> {
        delete(guild::table).filter(guild::id.eq(gid)).execute(conn)
    }
}
//
impl MakeBottle {
    pub fn make(&self, conn:&Conn) -> Res<Bottle> {
        insert_into(bottle::table).values(self).get_result(conn)
    }
}

impl Bottle {
    pub fn get(id:BottleId, conn:&Conn) -> Res<Self> {
        bottle::table.find(id).get_result(conn)
    }

    pub fn delete(id:BottleId, conn:&Conn) -> Res<usize> {
        delete(bottle::table).filter(bottle::id.eq(id)).execute(conn)
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

    pub fn get_from_message(mid:i64, conn:&Conn) -> Res<Self> {
        guild_bottle::table.filter(guild_bottle::message.eq(mid)).get_result(conn)
    }

    pub fn delete(buid:GuildBottleId, conn:&Conn) -> Res<usize> {
        delete(guild_bottle::table.find(buid)).execute(conn)
    }
}

//impl Report {
//    fn make(&self, conn:&Conn) -> Res<ReportId> {
//
//    }
//
//    fn get_from_message(mid:i64, conn:&Conn) -> Res<(ReportId, Self)> {
//
//    }
//
//    fn del(rid:ReportId) -> Res<()> {
//
//    }
//}

pub fn get_bottle_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("bottle".to_owned())).get_result(conn)
}

pub fn get_user_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("user".to_owned())).get_result(conn)
}

pub fn get_guild_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("guild".to_owned())).get_result(conn)
}