use model::*;

use diesel::prelude::*;
use std::error::Error;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};
use diesel::pg::PgConnection;
use schema::*;
use diesel::*;
use diesel::dsl::*;

use serenity::model as dc;

type Res<A> = Result<A, Error>;

trait OwnsTable<TABLE: query_source::Table> {
    const TABLE: TABLE;
}

pub trait CrudOp<Id> where Self: OwnsTable {
    fn make(&self, id: Id, conn:&Conn) -> Res<Id> {
        insert_into(Self::TABLE).values(&self).execute(conn)
    }

    fn get(id: Id, conn:&Conn) -> Res<Self> {
        select(Self::TABLE.find(id)).first(conn)
    }

    fn update(&self, id: Id, conn:&Conn) -> Res<()> {
        update(Self::TABLE.find(id)).set(&self).execute(conn)
    }

    fn delete(id: Id, conn:&Conn) -> Res<()> {
        delete(Self::TABLE.find(id)).execute(conn)
    }
}

impl OwnsTable for User { const TABLE: query_source::Table = user::table; }
impl OwnsTable for Guild { const TABLE: query_source::Table = guild::table; }
impl OwnsTable for Bottle { const TABLE: query_source::Table = bottle::table; }
impl OwnsTable for BottleUser { const TABLE: query_source::Table = bottle_user::table; }
impl OwnsTable for Report { const TABLE: query_source::Table = report::table; }

impl CrudOp<UserId> for User {
    fn get(uid: UserId, conn:&Conn) -> Res<Self> {
        insert_into(user::table)
            .values(&User::new(uid))
            .on_conflict(user::userid)
            .find(uid)
    }
}

impl CrudOp<GuildId> for Guild {}
impl CrudOp<BottleId> for Bottle {}
impl CrudOp<BottleUserId> for BottleUser {}
impl CrudOp<ReportId> for Report {}

//impl User {
//    fn get(uid: UserId, conn:&Conn) -> Res<Self> {
//        insert_into(user::table)
//            .values(&User::new(uid))
//            .on_conflict(user::userid)
//            .find(uid)
//    }
//
//    fn update(&self, uid: UserId, conn:&Conn) -> Res<()> {
//        update(user::table.find(uid)).set(self).execute(conn)
//    }
//
//    fn get_last_bottle(uid: UserId, conn:&Conn) -> Res<Bottle> {
//        bottle::table.filter(bottle::user.eq(uid)).first(conn)
//    }
//}
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
//impl Bottle {
//    fn make(&self, conn:&Conn) -> Res<BottleId> {
//
//    }
//
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
//}
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