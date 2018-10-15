use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};
use chrono::NaiveDateTime;

use serenity::prelude::*;
use serenity::framework::standard::StandardFramework;
use serenity::model::channel::{Message, Channel};
use serenity::model::event::*;
use serenity::model::id::*;

use super::schema::*;

pub type ConnPool = Pool<ConnectionManager<PgConnection>>;
pub type Conn = PooledConnection<ConnectionManager<PgConnection>>;
pub type DTime = NaiveDateTime;

pub type BottleId = i64;
pub type GuildId = i64;
pub type UserId = i64;
pub type BottleUserId = (BottleId, UserId);
pub type ReportId = i64;
#[derive(Insertable)]
#[table_name="bottle"]
pub struct MakeBottle {
    pub user: UserId,
    pub messageid: i64,
    pub time_pushed: DTime,
    
    pub reply_to: Option<BottleId>,
    pub message: String
}

#[derive(Queryable, Identifiable)]
#[table_name="bottle"]
pub struct Bottle {
    pub id: BottleId,

    pub user: UserId,
    pub messageid: i64,
    pub time_pushed: DTime,

    pub reply_to: Option<BottleId>,
    pub message: String
}

#[derive(Queryable, Insertable, Identifiable)]
#[table_name="user"]
pub struct User {
    pub id: UserId,
    pub subscribed: bool,
    pub xp: i64
}

#[derive(Queryable, Insertable)]
#[table_name="bottle_user"]
pub struct BottleUser {
    pub bottle: BottleId,
    pub user: UserId,
    pub time_recieved: DTime
}

impl User {
    pub fn new (uid: UserId) -> User {
        User {id: uid, subscribed: true, xp: 0}
    }
}

#[derive(Queryable, Insertable, Identifiable)]
#[table_name="guild"]
pub struct Guild {
    pub id: GuildId,
    pub admin_channel: i64
}

#[derive(Insertable)]
#[table_name="report"]
pub struct MakeReport {
    pub bottle: BottleId,
    pub guild: GuildId,
    pub user: UserId,
}

#[derive(Queryable, Identifiable)]
#[table_name="report"]
pub struct Report {
    pub id: ReportId,
    pub bottle: BottleId,
    pub guild: GuildId,
    pub user: UserId,
}

#[derive(Clone)]
pub struct Config {pub token:String, pub client_id: String, pub client_secret: String, pub database_path:String}

pub type Res<A> = Result<A, Box<std::error::Error>>;