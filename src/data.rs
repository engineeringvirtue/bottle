use model::*;

use diesel::prelude::*;
use std::error::Error;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};
use diesel::pg::PgConnection;
use schema::*;
use diesel::*;
use diesel::pg::upsert::*;
use diesel::dsl::*;

use serenity::model as dc;

type Res<A> = Result<A, result::Error>;

impl User {
    pub fn get(uid: UserId, conn:&Conn) -> Res<Self> {
        Ok(match user::table.find(uid).first(conn) {
            Ok(x) => x, _ => User::new(uid)
        })
    }

//    pub fn update(&self, uid: UserId, conn:&Conn) -> Res<()> {
//        insert_into(user::table).values(self).execute(conn)
//    }
//
//    pub fn get_last_bottle(uid: UserId, conn:&Conn) -> Res<Bottle> {
//        select(bottle::table.filter(bottle::user.eq(uid))).first(conn)
//    }
//
//    pub fn bottles_pending(uid:UserId, conn:&Conn) -> Res<bool> {
//        select(exists(bottle_user::table.filter(bottle_user::user.eq(uid)))).get_result(conn)
//    }
}
//
//impl Guild {
//    fn get(gid: GuildId, conn:&Conn) -> Res<GuildId> {
//
//    }
//
//    fn update(&self, gid: GuildId, conn:&Conn) -> Res<()> {
//
//    }
//
//    fn delete(gid: GuildId, conn:&Conn) -> Res<()> {
//
//    }
//}
//
impl MakeBottle {
    pub fn make(&self, conn:&Conn) -> Res<Bottle> {
        insert_into(bottle::table).values(self).get_result(conn)
    }

//    fn get(bid: BottleId, conn:&Conn) -> Res<Self> {
//
//    }
//
//    fn get_from_message(mid: i64, conn:&Conn) -> Res<Self> {
//
//    }
//
//    fn update(&self, bid:BottleId, conn:&Conn) -> Res<()> {
//
//    }
}
//
//impl BottleUser {
//    fn make(&self, conn:&Conn) -> Res<BottleUserId> {
//
//    }
//
//    fn get(buid:BottleUserId, conn:&Conn) -> Res<Self> {
//
//    }
//
//    fn get_from_message(mid:i64, conn:&Conn) -> Res<Self> {
//
//    }
//
//    fn del(buid:BottleUserId, conn:&Conn) -> Res<()> {
//
//    }
//}
//
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
    bottle::table.select(count_star()).first(conn)
}

pub fn get_user_count (conn: &Conn) -> Res<i64> {
    user::table.select(count_star()).first(conn)
}

pub fn get_guild_count (conn: &Conn) -> Res<i64> {
    guild::table.select(count_star()).first(conn)
}