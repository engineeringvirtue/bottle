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

pub type BottleId = MessageId;  
#[derive(Queryable, Insertable)]
#[table_name="bottle"]
pub struct Bottle {
    pub user: i64,
    pub messageid: i64,
    pub time_pushed: DTime,
    
    pub reply_to: Option<i64>,
    pub message: String
}

#[derive(Queryable, Insertable)]
#[table_name="user"]
pub struct User {
    pub userid: i64,
    pub subscribed: bool,
    pub xp: i64
}

impl User {
    pub fn new (uid: i64) -> User {
        User {userid: uid, subscribed: true, xp: 0}
    }
}

#[derive(Queryable)]
pub struct Guild {
    pub guildid: i64,
    pub admin_channel: i64
}

#[derive(Queryable)]
pub struct BottleReport {
    pub bottle: BottleId,
    pub guild: i64,
    pub user: i64,
}

pub struct Config {pub token:String, pub client_id: String, pub client_secret: String, pub database_path:String}
