#![feature(try_blocks)]

extern crate r2d2;
#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate serenity;
extern crate json;
#[macro_use]
extern crate nickel;
extern crate oauth2;

pub mod schema;
pub mod data;
#[macro_use]
pub mod model;
pub mod web;
pub mod bottle;

use std::thread;
use std::fs::read_to_string;
use std::sync::{Arc};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use dotenv::dotenv;

use serenity::prelude::*;
use serenity::framework::standard::{Args, DispatchError, StandardFramework, HelpBehaviour, CommandOptions, help_commands};
use serenity::model::channel::{Message, Channel};
use serenity::model::event::*;
use serenity::model::id::*;

use schema::*;
use data::*;
use model::*;

const PUSHXP: i64 = 120;
const KEEPXP: i64 = 75;
const COOLDOWN: i32 = 100;

fn handle_err<F: Fn() -> Res<String>> (replymsg: &Message, handler: F) {
    match handler() {
        Ok(x) => replymsg.reply(&x),
        Err(x) => replymsg.reply(&x.to_string())
    }.unwrap();
}

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
                let msgid = *new_message.id.as_u64() as i64;

                handle_err(&new_message, || -> Res<String> {
                    let conn = self.db.get()?;

                    let user = User::get(userid, &conn)?;
                    let bottle = MakeBottle { messageid: msgid, reply_to: None, user: user.id, time_pushed: std::time::SystemTime::now(), message: new_message.content.clone() };

                    bottle::distribute_bottle(bottle, &ctx, &conn)?;

                    Ok ("Your message has been ~~discarded~~ pushed into the dark seas of discord!".to_string())
                });

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
    let config = get_config("config.json".to_owned()).unwrap();
    let manager = ConnectionManager::<PgConnection>::new(config.clone().database_path);
    let db = r2d2::Pool::builder().build(manager).expect("Error initializing connection pool.");

    let webdb = db.clone(); let webcfg = config.clone();
    thread::spawn( move || web::start_serv(webdb, webcfg));

    let mut client = Client::new(&config.token, Handler {db: db.clone()}).expect("Error initializing client.");

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("-"))
            .customised_help(help_commands::with_embeds, |c| {
                c.individual_command_tip("DM me your message to send it in a bottle to random people in random discord! Administrators, go to the site to change the channel where reports go.")
            })
    );

    client.start_autosharded().unwrap();
}