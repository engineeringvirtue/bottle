use std::thread;
use chrono::{DateTime, Utc};
use serenity::prelude::*;
use serenity::model::misc::EmojiIdentifier;
use serenity::model::id::{ChannelId, UserId, GuildId, EmojiId, MessageId};
use serenity::model::channel::{Message, ReactionType, Reaction};
use serenity::CACHE;
use diesel::prelude::*;
use diesel::expression::Expression;
use serenity::utils::Colour;
use diesel::sql_types;

use model::id::*;
use model;
use model::*;
use data::*;
use data::functions::random;
use schema::{guild};

pub fn level_to_col (lvl: usize) -> Colour {
    match lvl%8 {
        0 => Colour::BLURPLE,
        1 => Colour::BLUE,
        2 => Colour::TEAL,
        3 => Colour::DARK_GREEN,
        4 => Colour::KERBAL,
        5 => Colour::GOLD,
        6 => Colour::DARK_RED,
        _ => Colour::MAGENTA
    }
}

pub fn render_bottle (bottle: &Bottle, level: usize, channel: ChannelId, cfg:&Config) -> Res<Message> {
    let msg = channel.send_message(|x| x.embed(|e| {
        let title = if level > 0 { "You have found a message glued to the bottle!" } else { "You have recovered a bottle!" }; //TODO: better reply system, takes last bottle as an argument

        let mut e = e.title(title)
            .description(bottle.contents.clone())
            .timestamp(&DateTime::<Utc>::from_utc(bottle.time_pushed, Utc))
            .color(level_to_col(level))
            .field("Report", format!("{}/report/{}", cfg.host_url, bottle.id), false)
            .footer(|footer|
                if let Some(ref guild) = bottle.guild.and_then(|guild| GuildId(guild as u64).to_partial_guild().ok()) {
                    let mut f = footer.text(&guild.name);
                    if let Some(ref icon) = guild.icon_url() {
                        f = f.icon_url(&icon);
                    }

                    f
                } else {
                    footer.text("No guild found")
                }
            )
            .author(|author| {
                if bottle.guild.is_some() {
                    let user = UserId(bottle.user as u64).to_user();
                    let username = user.as_ref().map(|u| u.tag())
                        .unwrap_or("Error fetching username".to_owned());

                    let avatar = user.as_ref().ok().and_then(|u| u.avatar_url()).unwrap_or(ERROR_AVATAR.to_owned());

                    author.url(&format!("{}/{}", cfg.host_url, bottle.user.to_string()))
                        .name(&username).icon_url(&avatar)
                } else {
                    author.name("Anonymous").icon_url(&ANONYMOUS_AVATAR)
                }
            });

        if let Some(ref img) = bottle.image {
            e = e.image(img);
        }

        if let Some(ref url) = bottle.url {
            e = e.url(url);
        }

        e
    }))?;

    Ok(msg)
}

pub fn distribute_to_guild(bottles: &Vec<Bottle>, guild: Guild, conn: &Conn, cfg:&Config) -> Res<()> {
    let bottlechannelid = ChannelId(guild.bottle_channel.ok_or("No bottle channel")? as u64);

    for (i, bottle) in bottles.iter().rev().enumerate() {
        let msg = render_bottle(bottle, i, bottlechannelid, cfg)?;
        MakeGuildBottle {bottle: bottle.id, guild: guild.id, message: msg.id.as_i64(), time_recieved: now()}.make(conn)?;
    }

    Ok (())
}

pub fn distribute_bottle (bottle: MakeBottle, conn:&Conn, cfg:&Config) -> Res<()> {
    let bottle = bottle.make(conn)?;

    let mut query = guild::table.into_boxed();

    if let Some(guild) = bottle.guild {
        query = query.filter(guild::id.ne(guild))
    }

    query.filter(guild::bottle_channel.is_not_null()).order(random).first(conn)
        .map_err(|err| -> Box<Error> { err.into() })
        .and_then(|guild: Guild| -> Res<()> {
            let bottles = bottle.get_reply_list(conn)?;
            distribute_to_guild(&bottles, guild, conn, cfg)?;

            Ok(())
        }).ok();

    Ok(())
}

pub fn report_bottle(bottle: Bottle, user: model::UserId, conn: &Conn, cfg: &Config) -> Res<Message> {
    let channel = ChannelId(cfg.admin_channel as u64);
    let user = UserId(user as u64).to_user()?;
    let msg = channel.say(&format!("REPORT FROM {}. USER ID {}, BOTTLE ID {}.", user.tag(), user.id, bottle.id))?;

    let bottlemsg: Message = render_bottle(&bottle, 0, channel, cfg)?;

    let ban = EmojiIdentifier {id: EmojiId(cfg.ban_emoji), name: "ban".to_owned()};
    let del = EmojiIdentifier {id: EmojiId(cfg.delete_emoji), name: "delete".to_owned()};

    msg.react(ban.clone())?;
    bottlemsg.react(ban.clone())?;
    bottlemsg.react(del.clone())?;

    let rwguild = channel.to_channel()?.guild().unwrap();
    let guild = rwguild.read().guild_id.as_i64();
    MakeGuildBottle {bottle: bottle.id, guild, message: bottlemsg.id.as_i64(), time_recieved: now()}.make(conn)?;

    Ok(msg)
}

pub fn del_bottle(bid: BottleId, conn:&Conn) -> Res<()> {
    for b in GuildBottle::get_from_bottle(bid, conn)? {
        let guild = Guild::get(b.guild, conn);
        if let Some(mut msg) = guild.bottle_channel.and_then(|bchan| ChannelId(bchan as u64).message(MessageId(b.message as u64)).ok()) {
            let _ = msg.edit(|x| x.embed(|x| x.title("DELETED").description("This bottle has been deleted by an admin.")));
        }
    }

    Bottle::del(bid, conn)?;
    Ok(())
}

pub fn react(conn: &Conn, r: Reaction, add: bool, cfg: Config) -> Res<()> {
    let mid = r.message_id.as_i64();

    let user = User::get(r.user_id.as_i64(), conn);
    let emojiid = match r.emoji {
        ReactionType::Custom {id: emojiid, ..} => emojiid.as_u64().clone(),
        _ => return Ok (())
    };

    if user.admin {
        if let Ok(gbottle) = GuildBottle::get_from_message(mid, conn) {
            let bottle = Bottle::get(gbottle.bottle, conn)?;

            if emojiid == cfg.ban_emoji {
                let b = Ban {user: bottle.user, report: None};
                if add { b.make(conn)?; } else { b.del(conn)?; }
            } else if emojiid == cfg.delete_emoji && add {
                del_bottle(bottle.id, conn)?;
            }
        } else if let Ok(report) = Report::get_from_message(mid, conn) {
            if emojiid == cfg.ban_emoji {
                let b = Ban {user: report.user, report: Some(report.bottle)};
                if add { b.make(conn)?; } else { b.del(conn)?; }
            }
        }
    }

    Ok(())
}