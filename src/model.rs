use std;
use serenity;
use typemap::Key;
use chrono;
use oauth2;
pub use std::error::Error;
use uuid::Uuid;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};
use std::sync::Arc;

use super::schema::*;

pub const REPLY_PREFIX: &str = "->";
pub const PUSHXP: i32 = 15;
pub const REPLYXP: i32 = 65;
pub const URLXP: i32 = 2;
pub const IMAGEXP: i32 = 6;
pub const REPORTXP: i32 = 20;
pub const COOLDOWN: i64 = 10;
pub const MAX_TICKETS: i32 = 5;
pub const MIN_CHARS: usize = 10;

pub type ConnPool = Pool<ConnectionManager<PgConnection>>;
pub type Conn = PooledConnection<ConnectionManager<PgConnection>>;
pub type DTime = chrono::NaiveDateTime;

pub type BottleId = i64;
pub type GuildId = i64;
pub type UserId = i64;
pub type GuildBottleId = i64;
pub type GuildContributionId = (GuildId, UserId);
pub type ReportId = i64;

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

#[derive(Queryable, Insertable, AsChangeset, Identifiable, Clone)]
#[table_name="user"]
pub struct User {
    pub id: UserId,
    pub session: Option<Uuid>,
    pub xp: i32,
    pub admin: bool,
    pub tickets: i32
}

#[derive(Queryable, Associations, Identifiable, Clone)]
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
    pub invite: Option<String>,
    pub bottle_channel: Option<i64>,
    pub admin_channel: Option<i64>,
    pub prefix: Option<String>
}

impl Guild {
    pub fn new (gid: GuildId) -> Guild {
        Guild {id: gid, bottle_channel: None, invite: None, admin_channel: None, prefix: None}
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

#[derive(Queryable, Insertable, AsChangeset)]
#[table_name="guild_contribution"]
pub struct GuildContribution {
    pub guild: GuildId,
    pub user: UserId,
    pub xp: i32
}

impl User {
    pub fn new (uid: UserId) -> User {
        User {id: uid, session: None, xp: 0, admin: false, tickets: 0}
    }
}

#[derive(Queryable, Insertable)]
#[table_name="report"]
pub struct Report {
    pub bottle: BottleId,
    pub message: i64,
    pub user: UserId
}

#[derive(Queryable, Insertable)]
#[table_name="ban"]
pub struct Ban {
    pub report: Option<ReportId>,
    pub user: UserId
}

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub token: String,
    pub discord_bots_token: String,
    pub client_id: String,
    pub client_secret: String,
    pub database_url: String,
    pub host_url: String,
    pub admin_channel: i64,
    pub ban_emoji: u64,
    pub delete_emoji: u64,
    pub auto_admin: UserId,
    pub cookie_sig: String
}

pub type Res<A> = Result<A, Box<Error>>;

pub fn now() -> chrono::NaiveDateTime {
    chrono::offset::Utc::now().naive_utc()
}

pub fn user_url(uid: UserId, cfg: &Config) -> String {
    format!("{}/u/{}", cfg.host_url, uid)
}

pub fn guild_url(gid: GuildId, cfg: &Config) -> String {
    format!("{}/g/{}", cfg.host_url, gid)
}

pub fn anonymous_url(cfg: &Config) -> String {
    format!("{}/img/anonymous.png", cfg.host_url)
}

pub fn error_url(cfg: &Config) -> String {
    format!("{}/img/fetcherror.png", cfg.host_url)
}

pub fn report_url(bid: BottleId, cfg: &Config) -> String { format!("{}/report/{}", cfg.host_url, bid) }

pub struct DConfig;
impl Key for DConfig {
    type Value = Config;
}

pub struct DOauth2;
impl Key for DOauth2 {
    type Value = oauth2::Config;
}

pub struct DBots;
impl Key for DBots {
    type Value = Arc<discord_bots::Client>;
}

pub trait GetConfig {
    fn get_cfg(&self) -> Config;
}

impl GetConfig for serenity::prelude::Context {
    fn get_cfg(&self) -> Config {
        self.data.lock().get::<DConfig>().unwrap().clone()
    }
}

pub struct DConn;
impl Key for DConn {
    type Value = ConnPool;
}

pub trait GetConnection {
    fn get_conn(&self) -> Conn {
        self.get_pool().get_conn()
    }

    fn get_pool(&self) -> ConnPool;
}

impl GetConnection for Pool<ConnectionManager<PgConnection>> {
    fn get_conn(&self) -> Conn {
        self.get().unwrap()
    }

    fn get_pool(&self) -> ConnPool {
        self.clone()
    }
}

impl GetConnection for serenity::prelude::Context {
    fn get_pool(&self) -> ConnPool {
        self.data.lock().get::<DConn>().unwrap().get_pool()
    }
}

pub trait GetBots {
    fn get_bots(&self) -> Arc<discord_bots::Client>;
}

impl GetBots for serenity::prelude::Context {
    fn get_bots(&self) -> Arc<discord_bots::Client> {
        self.data.lock().get::<DBots>().unwrap().clone()
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

    impl AsI64 for EmojiId {
        fn as_i64(&self) -> i64 {
            *self.as_u64() as i64
        }
    }
}