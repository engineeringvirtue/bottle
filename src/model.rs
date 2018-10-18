use std;
use serenity;
use typemap::Key;
use chrono;
pub use std::error::Error;
use uuid::Uuid;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};

use super::schema::*;

pub const REPLY_PREFIX: &str = "->";
pub const PUSHXP: i32 = 120;
pub const REPLYXP: i32 = 75;
pub const COOLDOWN: i64 = 45;

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
    pub contents: String,
    pub url: Option<String>,
    pub image: Option<String>
}

#[derive(Queryable, Insertable, AsChangeset, Identifiable)]
#[table_name="user"]
pub struct User {
    pub id: UserId,
    pub session: Option<Uuid>,
    pub token: Option<String>,
    pub xp: i32,
    pub admin: bool
}

#[derive(Queryable, Associations, Identifiable)]
#[table_name="bottle"]
#[belongs_to(User, foreign_key="user")]
#[belongs_to(Bottle, foreign_key="reply_to")]
pub struct Bottle {
    pub id: BottleId,
    pub reply_to: Option<BottleId>,

    pub user: UserId,
    pub message: i64,
    pub guild: Option<GuildId>,
    pub time_pushed: DTime,

    pub contents: String,
    pub url: Option<String>,
    pub image: Option<String>
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
    pub message: i64,
    pub time_recieved: DTime
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
        User {id: uid, session: None, token: None, xp: 0, admin: false}
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

pub type Res<A> = Result<A, Box<Error>>;

pub fn now() -> chrono::NaiveDateTime {
    chrono::offset::Utc::now().naive_utc()
}

pub struct DConn;
impl Key for DConn {
    type Value = ConnPool;
}

pub trait GetConnection { fn get_conn(&self) -> Conn; fn get_pool(&self) -> ConnPool; }

impl GetConnection for Pool<ConnectionManager<PgConnection>> {
    fn get_conn(&self) -> Conn {
        self.get().unwrap()
    }

    fn get_pool(&self) -> ConnPool {
        self.clone()
    }
}

impl GetConnection for serenity::prelude::Context {
    fn get_conn(&self) -> Conn {
        self.get_pool().get_conn()
    }

    fn get_pool(&self) -> ConnPool {
        self.data.lock().get::<DConn>().unwrap().get_pool()
    }
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