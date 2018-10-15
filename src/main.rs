extern crate r2d2;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate chrono;
extern crate serenity;
extern crate json;
#[macro_use]
extern crate nickel;
extern crate oauth2;

pub mod schema;
pub mod data;
pub mod model;
pub mod web;

use chrono::prelude::*;
use std::thread;
use std::fs::read_to_string;
use std::sync::{Arc};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use dotenv::dotenv;

use serenity::prelude::*;
use serenity::framework::standard::StandardFramework;
use serenity::model::channel::{Message, Channel};
use serenity::model::event::*;
use serenity::model::id::*;

use schema::*;
use model::*;

const PUSHXP: i64 = 120;
const KEEPXP: i64 = 75;
const COOLDOWN: i32 = 100;

struct Handler {pub db: ConnPool}
impl EventHandler for Handler {
    fn ready(&self, ctx:Context, data_about_bot: serenity::model::gateway::Ready) {
        let pfp = serenity::utils::read_image("./assets/icon.png").unwrap();
        ctx.edit_profile(|p| p.avatar(Some(&pfp))).unwrap();
    }
    
    fn message(&self, ctx: Context, new_message: Message) {
        match new_message.channel() {
            Some(Channel::Private(ref channel)) if !new_message.author.bot => {
                let channel = channel.read();
                let userid = *new_message.author.id.as_u64() as i64;
                let bottle = Bottle {messageid: *new_message.id.as_u64() as i64, reply_to: None, user: userid, time_pushed: Utc::now().naive_utc(), message: new_message.content};

                let conn = self.db.get().unwrap();
                diesel::insert_into(user::table).values(User::new(userid)).on_conflict_do_nothing().execute(&conn); //create user
                diesel::insert_into(bottle::table).values(&bottle).execute(&conn).expect("Error making bottle"); //insert bottle

                channel.say("Your message has been ~~discarded~~ pushed into the dark seas of discord!").unwrap();
            }, _ => ()
        }
    }

    fn message_update(&self, ctx: Context, new_data: MessageUpdateEvent) {
        //TODO: support message edits and deletion
    }
}

fn get_config (path: String) -> Result<Config, Box<std::error::Error>> {
    let cfgstr = read_to_string(path)?;
    let cfg = json::parse(&cfgstr)?;
    Ok (Config {token: cfg["token"].as_str().ok_or("Token not found!")?.to_string(), client_id: cfg["client_id"].as_str().ok_or("Client id not found!")?.to_string(), client_secret: cfg["client_secret"].as_str().ok_or("Client secret not found!")?.to_string(), database_path: cfg["database-path"].as_str().ok_or("Database path not found!")?.to_string()})
}

fn main() {
    let config = Arc::new(get_config("config.json".to_owned()).unwrap());
    let manager = ConnectionManager::<PgConnection>::new(config.database_path);
    let db = r2d2::Pool::builder().build(manager).expect("Error initializing connection pool.");

    let webdb = db.clone(); let webcfg = Arc::clone(&config);
    thread::spawn( move || web::start_serv(webdb, webcfg));

    let mut client = Client::new(&config.token, Handler {db: db.clone()}).expect("Error initializing client.");

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("-"))
    );

    client.start_autosharded().unwrap();
}
