#![feature(try_blocks)]
#![allow(proc_macro_derive_resolution_fallback)]

extern crate chrono;
extern crate r2d2;
#[macro_use]
extern crate diesel;
extern crate dotenv;
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
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;

use serenity::prelude::*;
use serenity::framework::standard::{Args, DispatchError, StandardFramework, HelpBehaviour, CommandOptions, help_commands};
use serenity::model::channel::{Message, Channel};
use serenity::model::event::*;

use data::*;
use model::*;
use model::id::*;

fn handle_err<F: Fn() -> Res<String>> (replymsg: &Message, handler: F) -> Res<String> {
    let res = handler();
    match &res {
        Ok(x) => replymsg.reply(&x),
        Err(x) => replymsg.reply(&x.to_string())
    };

    res
}

struct Handler {pub db: ConnPool}
impl EventHandler for Handler {
    fn ready(&self, _ctx:Context, _data_about_bot: serenity::model::gateway::Ready) {
        println!("Ready!");
    }

    fn message(&self, ctx: Context, new_message: Message) {
        let conn = self.db.get().unwrap();

        let new_bottle = |guild| {
            handle_err (&new_message, || {
                let userid = new_message.author.id.as_i64();
                let msgid = new_message.id.as_i64();

                //TODO: check cooldown
                let mut user = User::get(userid, &conn);
                user.xp += PUSHXP;
                user.update(&conn)?;

                let bottle = MakeBottle { message: msgid, reply_to: None, guild, user: user.id, time_pushed: now(), contents: new_message.content.clone() };

                bottle::distribute_bottle(bottle, &ctx, &conn)?;

                Ok ("Your message has been ~~discarded~~ pushed into the dark seas of discord!".to_string())
            });
        };

        match new_message.channel() {
            Some(Channel::Guild(ref channel)) => {
                let channel = channel.read();
                let gid = channel.guild_id.as_i64();
                let guilddata = Guild::get(gid, &conn);

                if Some(channel.id.as_i64()) == guilddata.bottle_channel {
                    new_bottle(Some(gid))
                }
            },

            Some(Channel::Private(ref channel)) if !new_message.author.bot => new_bottle(None)
            , _ => ()
        };
    }

    fn message_update(&self, _ctx: Context, _new_data: MessageUpdateEvent) {
        //TODO: support message edits and deletion
    }

    fn guild_create (&self, _ctx: Context, guild: serenity::model::guild::Guild, _is_new: bool) {
        let conn = &self.db.get().unwrap();
        Guild::get(guild.id.as_i64(), &conn).update(&conn).unwrap();
    }

    fn guild_delete (&self, _ctx: Context, incomplete: serenity::model::guild::PartialGuild, _full: Option<Arc<RwLock<serenity::model::guild::Guild>>>) {
        Guild::delete(incomplete.id.as_i64(), &self.db.get().unwrap()).unwrap();
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
            .help(|f, msg, opts, cmds, args | {
                msg.reply("DM me your message to send it in a bottle to random people in random discord! Administrators, go to the site to change the channel where reports go.")?;

                Ok(())
            })
    );

    client.start_autosharded().unwrap();
}