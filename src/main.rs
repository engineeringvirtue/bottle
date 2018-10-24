#![allow(proc_macro_derive_resolution_fallback)]
extern crate time;
extern crate chrono;
extern crate r2d2;
extern crate uuid;
#[macro_use]
extern crate diesel;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate kankyo;
extern crate envy;
extern crate typemap;
#[macro_use]
extern crate serenity;

extern crate iron;
extern crate handlebars_iron;
extern crate staticfile;
extern crate mount;
extern crate router;
extern crate params;
extern crate oauth2;
extern crate reqwest;
extern crate bincode;
extern crate iron_sessionstorage_0_6;

pub mod schema;
pub mod data;
#[macro_use]
pub mod model;
pub mod web;
pub mod bottle;

use std::thread;
use std::fs::File;
use std::sync::{Arc};
use time::Duration;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;

use serenity::prelude::*;
use serenity::framework::standard::{Args, CommandError, DispatchError, StandardFramework, HelpBehaviour, CommandOptions, help_commands};
use serenity::model::channel::{Message, Channel, Embed, Attachment, Reaction};
use serenity::model::event::*;
use serenity::model::gateway;
use serenity::model::permissions::Permissions;
use serenity::CACHE as cache;

use data::*;
use model::*;
use model::id::*;

const ADMIN_PERM: Permissions = Permissions::ADMINISTRATOR;

fn handle_ev_err<F: Fn() -> Res<String>> (replymsg: &Message, handler: F) {
    match handler() {
        Ok(x) => replymsg.reply(&x).ok(),
        Err(x) => replymsg.reply(&x.to_string()).ok()
    };
}

struct Handler;
impl EventHandler for Handler {
    fn message(&self, ctx: Context, new_message: Message) {
        if !new_message.author.bot {
            let conn = ctx.get_conn();

            let new_bottle = |guild| {
                handle_ev_err(&new_message, || {
                    let userid = new_message.author.id.as_i64();
                    let msgid = new_message.id.as_i64();
                    let cfg = ctx.get_cfg();

                    let mut user = User::get(userid, &conn);
                    let lastbottle = user.get_bottle(&conn).ok();
                    match lastbottle {
                        Some (ref bottle) => {
                            let since_push = now().signed_duration_since(bottle.time_pushed);
                            let cooldown = Duration::minutes(COOLDOWN);
                            if since_push < cooldown && !user.admin {
                                let towait = cooldown - since_push;
                                return Err(format!("You must wait {} minutes before sending another bottle!", towait.num_minutes()).into())
                            }
                        },
                        _ => ()
                    }

                    if !user.admin && user.get_banned(&conn)? {
                        return Err("You are banned from using Bottle! Appeal by dming the global admins!".into());
                    }

                    let mut contents = new_message.content.clone();
                    let replyto = match guild {
                        Some (g) if (&contents).starts_with(REPLY_PREFIX) => {
                            contents = contents.chars().skip(REPLY_PREFIX.chars().count()).collect();
                            let gbottle = Guild::get(g, &conn).get_last_bottle(&conn).map_err(|_| "No bottle to reply found!")?;
                            Some (gbottle.bottle)
                        }, _ => None
                    };

                    let url = new_message.embeds.get(0).and_then(|emb: &Embed| emb.url.clone());
                    let image = new_message.attachments.get(0).and_then(|a: &Attachment| a.dimensions().map(|_| a.url.clone()));

                    user.xp += match replyto {Some(_) => REPLYXP, None => PUSHXP};

                    if let Some (_) = url {
                        user.xp += URLXP;
                    }
                    if let Some (_) = image {
                        user.xp += IMAGEXP;
                    }

                    user.update(&conn)?;

                    let bottle = MakeBottle { message: msgid, reply_to: replyto, guild, user: user.id, time_pushed: now(), contents, url, image };
                    let connpool = ctx.get_pool();
                    thread::spawn(move || {
                        bottle::distribute_bottle(bottle, &connpool.get_conn(), &cfg).ok();
                    });

                    Ok("Your message has been ~~discarded~~ pushed into the dark seas of discord!".to_owned())
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

                Some(Channel::Private(_)) => new_bottle(None)
                ,
                _ => ()
            };
        }
    }

    fn reaction_add(&self, ctx: Context, r: Reaction) {
        let conn = &ctx.get_conn();
        bottle::react(conn, r, true, ctx.get_cfg()).unwrap();
    }

    fn reaction_remove(&self, ctx: Context, r: Reaction) {
        let conn = &ctx.get_conn();
        bottle::react(conn, r, false, ctx.get_cfg()).unwrap();
    }

    fn message_update(&self, _ctx: Context, _new_data: MessageUpdateEvent) {
        //TODO: support message edits and deletion
    }

    fn guild_create (&self, ctx: Context, guild: serenity::model::guild::Guild, is_new: bool) {
        let conn = ctx.get_conn();
        let guilddata = Guild::get(guild.id.as_i64(), &conn);
        let user = &cache.read().user;

        if is_new {
            let general = guild.channels.iter()
                .find(|&(channelid, _)| guild.permissions_in(channelid, user).send_messages());

            if let Some((channel, _)) = general {
                channel.send_message(|x|
                    x.content("Hey! If you want to receive and send bottles, please set the channel you want to receive them in with ``-configure``. Thanks!")).ok();
            }
        }

        guilddata.update(&conn).unwrap();
    }

    fn guild_delete (&self, ctx: Context, incomplete: serenity::model::guild::PartialGuild, _full: Option<Arc<RwLock<serenity::model::guild::Guild>>>) {
        Guild::delete(incomplete.id.as_i64(), &ctx.get_conn()).ok();
    }

    fn ready(&self, ctx:Context, _data_about_bot: serenity::model::gateway::Ready) {
        ctx.set_presence(Some(gateway::Game {kind: gateway::GameType::Listening, name: "you, try -help".to_owned(), url: None})
                         , serenity::model::user::OnlineStatus::DoNotDisturb);

        let conn = &ctx.get_conn();
        let mut u = User::get(ctx.get_cfg().auto_admin, conn);
        u.admin = true;
        u.update(conn).ok();

        println!("Ready!");
    }
}

fn main() {
    kankyo::load_from_reader(&mut File::open("./.env").unwrap()).unwrap();
    let config:Config = envy::from_env::<Config>().unwrap();
    let manager = ConnectionManager::<PgConnection>::new(config.clone().database_url);
    let db = r2d2::Pool::builder().build(manager).expect("Error initializing connection pool.");

    let webdb = db.clone(); let webcfg = config.clone();
    thread::spawn( move || web::start_serv(webdb, webcfg));

    let mut client = Client::new(&config.token, Handler).expect("Error initializing client.");
    client.data.lock().insert::<DConn>(db.clone());
    client.data.lock().insert::<DConfig>(config.clone());

    client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("-")) // set the bot's prefix to "~"
        .help(|_f, msg, _opts, _cmds, _args | {
              msg.reply ("Set a bottle channel with ``-configure``, then start sending out and replying to bottles there! Or dm me for anonymous bottles! :^)")?;

              Ok(())
        })
        .command("configure", |c|
            c.required_permissions(ADMIN_PERM)
                .guild_only(true)
                .exec(| ctx, msg, mut args: serenity::framework::standard::Args | {
                    let chan = args.single::<serenity::model::channel::Channel>()
                        .map_err(|_| "Please specify a valid channel.")?;

                    let conn = ctx.get_conn();

                    let mut guild = Guild::get(msg.guild_id.unwrap().as_i64(), &conn);
                    guild.bottle_channel = Some(chan.id().as_i64());
                    guild.update(&conn)?;

                    msg.reply("All set!")?;
                    Ok (())
                })
        )
        .command("mote", |c|
            c.exec(|ctx, msg, mut args| {
                let usr = args.single::<serenity::model::user::User>()
                    .map_err(|_| "Please specify a user to promote.")?;

                if ctx.get_cfg().auto_admin == msg.author.id.as_i64() {
                    let conn = &ctx.get_conn();
                    let mut u = User::get(usr.id.as_i64(), conn);
                    if !u.admin {
                        u.admin = true;
                        msg.reply(&format!("Promoted {}", usr.tag()))?;
                    } else {
                        u.admin = false;
                        msg.reply(&format!("Demoted {}", usr.tag()))?;
                    }

                    u.update(conn)?;
                    Ok(())
                } else {
                    Err("You must be an auto admin to do this!".into())
                }
            })
        )
        .after(| _ctx, msg, _, res | {
            if let Err(CommandError(str)) = res {
                msg.reply(&str).ok();
            }
        })
    );

    client.start_autosharded().unwrap();
}