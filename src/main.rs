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

extern crate iron;
extern crate handlebars_iron;
extern crate staticfile;
extern crate mount;
extern crate router;
extern crate params;
extern crate oauth2;
extern crate reqwest;
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

const ADMIN_PERM: Permissions = Permissions::ADMINISTRATOR;

fn update_guilds(ctx: &Context) {
    let guild_count = {
        cache.read().all_guilds().len()
    };

    let stats = discord_bots::PostBotStats::new(discord_bots::ServerCount::Single(guild_count));
    let _ = ctx.get_bots().post_stats(stats);
}

struct Handler;
impl EventHandler for Handler {
    fn message(&self, ctx: Context, new_message: Message) {
        if !new_message.author.bot {
            let conn = ctx.get_conn();

            let res = match new_message.channel() {
                Some(Channel::Guild(ref channel)) => {
                    let channel = channel.read();
                    let gid = channel.guild_id.as_i64();
                    debug!("New message in guild {}, looking at db....", gid);
                    let guilddata = Guild::get(gid, &conn);

                    if Some(channel.id.as_i64()) == guilddata.bottle_channel {
                        bottle::new_bottle(&new_message, Some(gid), ctx.get_pool(), ctx.get_cfg())
                    } else {
                        Ok(None)
                    }
                },

                Some(Channel::Private(_)) => bottle::new_bottle(&new_message, None, ctx.get_pool(), ctx.get_cfg()),
                _ => Ok(None)
            };

            match res {
                Ok(Some(x)) => new_message.reply(&x).ok(),
                Err(x) => new_message.reply(x.description()).ok(),
                _ => None
            };
        }
    }

    fn message_delete(&self, ctx: Context, _channel: serenity::model::id::ChannelId, deleted_msg_id: serenity::model::id::MessageId) {
        debug!("Message {} deleted, checking db...", deleted_msg_id);
        let conn = &ctx.get_conn();
        if let Ok(x) = Bottle::get_from_message(deleted_msg_id.as_i64(), conn) {
            bottle::del_bottle(x, conn, &ctx.get_cfg()).unwrap();
        }
    }

    fn message_update(&self, ctx: Context, new_data: serenity::model::event::MessageUpdateEvent) {
        debug!("Message {:?} updated, checking db....", new_data);
        if let Ok(x) = new_data.channel_id.message(new_data.id) {
            if !x.author.bot {
                let res = bottle::edit_bottle(&x, x.guild_id.map(AsI64::as_i64), ctx.get_pool(), &ctx.get_cfg());

                match res {
                    Ok(Some(msg)) => x.reply(&msg).ok(),
                    Err(err) => x.reply(err.description()).ok(),
                    _ => None
                };
            }
        }
    }

    fn reaction_add(&self, ctx: Context, r: Reaction) {
        debug!("Reaction {:?} added...", r);
        let conn = &ctx.get_conn();
        bottle::react(conn, r, true, &ctx.get_cfg()).unwrap();
    }

    fn reaction_remove(&self, ctx: Context, r: Reaction) {
        debug!("Reaction {:?} removed...", r);
        let conn = &ctx.get_conn();
        bottle::react(conn, r, false, &ctx.get_cfg()).unwrap();
    }

    fn guild_create (&self, ctx: Context, guild: serenity::model::guild::Guild, is_new: bool) {
        let conn = ctx.get_conn();
        let guilddata = Guild::get(guild.id.as_i64(), &conn);
        let user = cache.read().user.id.clone();

        if is_new {
            let general = guild.channels.iter()
                .find(|&(channelid, _)| guild.permissions_in(channelid, user).send_messages());

            if let Some((channel, _)) = general {
                let _ = channel.send_message(|x|
                    x.content("Hey! If you want to receive and send bottles, please set the channel you want to receive them in with ``-configure``. Thanks!"));
            }
        }

        guilddata.update(&conn).unwrap();
        update_guilds(&ctx);
        info!("Gained guild {}.", &guild.name)
    }

    fn guild_delete (&self, ctx: Context, incomplete: serenity::model::guild::PartialGuild, _full: Option<Arc<RwLock<serenity::model::guild::Guild>>>) {
        Guild::del(incomplete.id.as_i64(), &ctx.get_conn()).unwrap();

        update_guilds(&ctx);
        info!("Guild lost.")
    }

    fn ready(&self, ctx:Context, _data_about_bot: serenity::model::gateway::Ready) {
        ctx.set_presence(Some(gateway::Game {kind: gateway::GameType::Listening, name: "you, try -help".to_owned(), url: None})
                         , serenity::model::user::OnlineStatus::Online);

        let conn = &ctx.get_conn();
        let mut u = User::get(ctx.get_cfg().auto_admin, conn);
        u.admin = true;
        let _ = u.update(conn);

        info!("Client is ready");
    }
}

#[cfg(not(debug_assertions))]
embed_migrations!("migrations/");

#[cfg(not(debug_assertions))]
fn do_migrations(db: &ConnPool) {
    embedded_migrations::run_with_output(&db.get_conn(), &mut std::io::stdout()).unwrap();
}

#[cfg(debug_assertions)]
fn do_migrations(_: &ConnPool) { }

fn main() {
    kankyo::load_from_reader(&mut File::open("./.env").unwrap()).unwrap();
    let config:Config = envy::from_env::<Config>().unwrap();
    let manager = ConnectionManager::<PgConnection>::new(config.clone().database_url);
    let db = r2d2::Pool::builder().build(manager).expect("Error initializing connection pool.");

    do_migrations(&db);

    let log_level = if config.debug_log { log::LevelFilter::Debug }
        else { log::LevelFilter::Error };

    simple_logging::log_to_stderr(log_level);

    let webdb = db.clone(); let webcfg = config.clone();
    thread::spawn( move || web::start_serv(webdb, webcfg));

    let dbots = Arc::new(discord_bots::Client::new(&config.discord_bots_token));

    let mut client = Client::new(&config.token, Handler).expect("Error initializing client.");
    client.data.lock().insert::<DBots>(dbots);
    client.data.lock().insert::<DConn>(db.clone());
    client.data.lock().insert::<DConfig>(config.clone());

    client.with_framework(StandardFramework::new()
        .configure(|c| c.on_mention(true)
            .prefix("-").dynamic_prefix(|ctx, msg| {
            let conn = &ctx.get_conn();

            msg.guild_id.and_then(|gid| Guild::get(gid.as_i64(), conn).prefix)
        }))
        .help(|_f, msg, _opts, _cmds, _args | {
              msg.reply ("Set a bottle channel with ``-configure <channel>``, then start sending out and replying (prefix your message with ``->`` to bottles there! Or dm me for anonymous bottles! :^) Also try ``-info``")?;

              Ok(())
        })
        .command("configure", |c|
            c.required_permissions(ADMIN_PERM)
                .guild_only(true)
                .exec(| ctx, msg, mut args: serenity::framework::standard::Args | {
                    let conn = &ctx.get_conn();
                    let mut guild = Guild::get(msg.guild_id.unwrap().as_i64(), &conn);

                    if let Ok(chan) = args.find::<serenity::model::channel::Channel>() {
                        guild.bottle_channel = Some(chan.id().as_i64());
                        guild.update(conn)?;

                        msg.reply("All set!")?;
                        Ok(())
                    } else if let Ok(x) = args.find::<char>() {
                        guild.prefix = Some(x.to_string());
                        guild.update(conn)?;

                        msg.reply(&format!("Set prefix to \"{}\"!", x))?;

                        Ok(())
                    } else {
                        Err("Please specify a valid channel or a single character prefix!".into())
                    }
                })
        )
        .group("Auto Admin Commands", |g|
            g.check(|ctx, msg, _args, _opts| {
                if ctx.get_cfg().auto_admin != msg.author.id.as_i64() {
                    let _ = msg.reply("You must be an auto admin to do this!");
                    false
                } else { true }
            })
            .command("mote", |c|
                c.exec(|ctx, msg, mut args| {
                    let usr = args.single::<serenity::model::user::User>()
                        .map_err(|_| "Please specify a user to promote.")?;

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
                })
            )
            .command("announce", |c|
                c.exec(|ctx, msg, args| {
                    let announcement = args.rest();

                    let conn = &ctx.get_conn();
                    for x in cache.read().all_guilds() {
                        if let Some(c) = Guild::get(x.as_i64(), conn).bottle_channel {
                            let cid = serenity::model::id::ChannelId(c as u64);
                            let _ = cid.send_message(|x| x.content(announcement));
                        }
                    }

                    info!("{} sent {} to all guilds!", msg.author.tag(), announcement);
                    msg.reply("Sent to all guilds!")?;
                    Ok(())
                })
            )
        )
        .command("info", |c|
            c.guild_only(true).exec(|ctx, msg, _args| {
                let conn = &ctx.get_conn();
                let gdata = Guild::get(msg.guild_id.unwrap().as_i64(), conn);
                let gdata_xp = gdata.get_xp(conn)?;

                let guild_channel = msg.channel().unwrap().guild().unwrap();
                let guild = guild_channel.read().guild().unwrap();

                guild_channel.read().send_message(|msg| msg.embed(|embed| {
                    let public = match gdata.invite.as_ref() {
                        Some(inv) => inv,
                        None => "Use -publicize to generate an invite!"
                    };

                    let bottle_channel = match gdata.bottle_channel.as_ref() {
                        Some(cid) => serenity::model::id::ChannelId(*cid as u64).mention(),
                        None => "Set with -configure <channel>".to_owned()
                    };

                    embed.title(guild.read().name.clone())
                        .field("Prefix", gdata.prefix.as_ref().map(String::as_str).unwrap_or_else(|| "Use -prefix to set a custom prefix"), true)
                        .field("XP", gdata_xp, true)
                        .field("Bottle channel", bottle_channel, true)
                        .field("Public", public, true)

                        .url(guild_url(gdata.id, &ctx.get_cfg()))
                }))?;

                Ok(())
            })
        )
        .command("publicize", |c|
            c.guild_only(true).required_permissions(ADMIN_PERM)
                .exec(|ctx, msg, _args| {
                let conn = &ctx.get_conn();
                let mut gdata = Guild::get(msg.guild_id.unwrap().as_i64(), conn);

                let guildc = msg.channel().unwrap().guild().unwrap();
                let inv = guildc.read().create_invite(|x| x.max_age(0).temporary(true))?;
                gdata.invite = Some(inv.url());
                gdata.update(conn)?;

                msg.reply("Guild publicized!")?;
                Ok(())
            })
        )

        .on_dispatch_error(| _ctx, msg, err | {
            match err {
                DispatchError::LackOfPermissions(_) => {
                    let _ = msg.reply("You lack permission to do this! Please make sure you are an administrator.");
                },
                _ => ()
            }
        })
        .after(|_ctx, msg, _, res| {
            if let Err(CommandError(str)) = res {
                let _ = msg.reply(&str);
            }
        })
    );

    client.start_autosharded().unwrap();
}