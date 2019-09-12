#![allow(proc_macro_derive_resolution_fallback)]
extern crate time;
extern crate chrono;
extern crate r2d2;
extern crate uuid;
#[macro_use]
extern crate diesel;
#[cfg(not(debug_assertions))]
#[macro_use]
extern crate diesel_migrations;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate kankyo;
extern crate envy;
extern crate typemap;
extern crate discord_bots;
extern crate serenity;

extern crate simple_logging;
#[macro_use]
extern crate log;

pub mod schema;
pub mod data;
#[macro_use]
pub mod model;
pub mod bottle;

use std::thread;
use std::collections::HashMap;
use std::string::ToString;
use std::fs::File;
use std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;

use serenity::prelude::*;
use serenity::framework::standard::{CommandError, DispatchError, StandardFramework};
use serenity::model::channel::{Message, Channel, Reaction};
use serenity::model::gateway;
use serenity::model::permissions::Permissions;
use serenity::CACHE as cache;

use model::*;
use model::id::*;
use std::time::Duration;

struct Handler;
impl EventHandler for Handler {}

fn main() {
    kankyo::load_from_reader(&mut File::open("./.env").unwrap()).unwrap();
    let config:Config = envy::from_env::<Config>().unwrap();
    let manager = ConnectionManager::<PgConnection>::new(config.clone().database_url);
    let db = r2d2::Pool::builder().build(manager).expect("Error initializing connection pool.");

    let log_level = if config.debug_log { log::LevelFilter::Debug }
        else { log::LevelFilter::Error };

    simple_logging::log_to_stderr(log_level);

    thread::spawn(move || {
        let mut client = Client::new(&config.token, Handler).expect("Error initializing client.");
        client.start_autosharded().unwrap();
    });

    let conn = db.get_conn();
    let channels = Arc::new(Guild::get_channels(&conn).unwrap());

    let mut num = 15;

    loop {
        for bottle in Bottle::get_range(num, 10, &conn).unwrap() {
            thread::spawn({
                let channels = channels.clone();

                move || {
                    let mut s = String::new();

                    s.push_str("**");

                    match &bottle.guild {
                        Some(_) => s.push_str(&serenity::model::id::UserId(bottle.user as u64).mention()),
                        None => s.push_str("anon")
                    }

                    s.push_str(&format!(" / {}:**\n", bottle.id.to_string()));

                    match &bottle.reply_to {
                        Some(id) => s.push_str(&format!(">>{} ", id)),
                        None => ()
                    }

                    s.push_str(&bottle.contents);

                    if let Some(url) = &bottle.url {
                        s.push(' ');
                        s.push_str(url);
                    }

                    if let Some(image) = &bottle.image {
                        s.push(' ');
                        s.push_str(image);
                    }

                    for channel in channels.iter() {
                        let channel = serenity::model::id::ChannelId(*channel as u64);
                        let _ = channel.send_message(|x| x.content(&s));
                    }
                }
            });
        }

        num += 10;
    }
}