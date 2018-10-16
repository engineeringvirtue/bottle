use std;
use chrono;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};

use super::schema::*;

pub const PUSHXP: i32 = 120;
pub const REPLYXP: i32 = 75;
pub const COOLDOWN: i32 = 100;
pub const DELETETIMEOUT: i32 = 720;

pub type ConnPool = Pool<ConnectionManager<PgConnection>>;
pub type Conn = PooledConnection<ConnectionManager<PgConnection>>;
pub type DTime = chrono::NaiveDateTime;

pub type BottleId = i64;
pub type GuildId = i64;
pub type UserId = i64;
pub type GuildBottleId = i64;

#[derive(Insertable)]
#[table_name="bottle"]
pub struct MakeBottle {
    pub user: UserId,
    pub message: i64,
    pub guild: Option<GuildId>,

    pub reply_to: Option<BottleId>,

    pub time_pushed: DTime,
    pub contents: String
}

#[derive(Queryable, Identifiable)]
#[table_name="bottle"]
pub struct Bottle {
    pub id: BottleId,
    pub reply_to: Option<BottleId>,

    pub user: UserId,
    pub message: i64,
    pub guild: Option<GuildId>,
    pub time_pushed: DTime,

    pub contents: String
}

#[derive(Queryable, Insertable, AsChangeset, Identifiable)]
#[table_name="user"]
pub struct User {
    pub id: UserId,
    pub subscribed: bool,
    pub token: Option<String>,
    pub xp: i32,
    pub admin: bool
}

#[derive(Queryable, Insertable, AsChangeset, Identifiable)]
#[table_name="guild"]
pub struct Guild {
    pub id: GuildId,
    pub bottle_channel: Option<i64>,
    pub admin_channel: Option<i64>
}

impl Guild {
    pub fn new (gid: GuildId) -> Guild {
        Guild {id: gid, bottle_channel: None, admin_channel: None}
    }
}

#[derive(Insertable)]
#[table_name="guild_bottle"]
pub struct MakeGuildBottle {
    pub bottle: BottleId,
    pub guild: GuildId,
    pub message: i64
}

#[derive(Queryable, Associations, Identifiable)]
#[belongs_to(Guild, foreign_key="guild")]
#[table_name="guild_bottle"]
pub struct GuildBottle {
    pub id: GuildBottleId,
    pub bottle: BottleId,
    pub guild: GuildId,
    pub message: i64,
    pub time_recieved: DTime
}

impl User {
    pub fn new (uid: UserId) -> User {
        User {id: uid, subscribed: true, token: None, xp: 0, admin: false}
    }
}

#[derive(Queryable, Insertable)]
#[table_name="report"]
pub struct Report {
    pub bottle: BottleId,
    pub user: UserId
}

#[derive(Queryable, Insertable)]
#[table_name="ban"]
pub struct Ban {
    pub report: BottleId,
    pub user: UserId
}

#[derive(Clone)]
pub struct Config {pub token:String, pub client_id: String, pub client_secret: String, pub database_path:String}

pub type Res<A> = Result<A, Box<std::error::Error>>;

pub fn now() -> chrono::NaiveDateTime {
    chrono::offset::Utc::now().naive_utc()
}

pub mod id {
    use serenity::model::id::*;

    pub trait AsI64 { fn as_i64(&self) -> i64; }

    impl AsI64 for UserId {
        fn as_i64(&self) -> i64 {
            *self.as_u64() as i64
        }
    }

    impl AsI64 for ChannelId {
        fn as_i64(&self) -> i64 {
            *self.as_u64() as i64
        }
    }

    impl AsI64 for GuildId {
        fn as_i64(&self) -> i64 {
            *self.as_u64() as i64
        }
    }

    impl AsI64 for MessageId {
        fn as_i64(&self) -> i64 {
            *self.as_u64() as i64
        }
    }
}