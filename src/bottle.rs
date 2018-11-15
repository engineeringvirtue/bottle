use std::thread;
use chrono::{DateTime, Utc};
use serenity::prelude::*;
use serenity::model::misc::EmojiIdentifier;
use serenity::model::id::{ChannelId, UserId, GuildId, EmojiId, MessageId};
use serenity::model::channel::{Message, ReactionType, Reaction, Embed, Attachment};
use serenity::CACHE;
use time::Duration;
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

pub fn col_wheel(num: usize) -> Colour {
    match num%8 {
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
            .color(col_wheel(level))
            .field("Report", report_url(bottle.id, cfg), true)
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
                        .unwrap_or_else(|_| "Error fetching username".to_owned());

                    let avatar = user.as_ref().ok().and_then(|u| u.avatar_url()).unwrap_or_else(|| error_url(cfg));

                    author.url(&user_url(bottle.user, cfg))
                        .name(&username).icon_url(&avatar)
                } else {
                    author.name("Anonymous").icon_url(&anonymous_url(cfg))
                }
            });

        if let Some(ref img) = bottle.image {
            e = e.image(img).url(img);
        }

        if let Some(ref url) = bottle.url {
            e = e.url(url);
        }

        e
    }))?;

    Ok(msg)
}

pub fn distribute_to_guild(bottles: &Vec<(usize, Bottle)>, guild: &Guild, conn: &Conn, cfg:&Config) -> Res<()> {
    let bottlechannelid = ChannelId(guild.bottle_channel.ok_or("No bottle channel")? as u64);

    let last_bottle = guild.get_last_bottle(conn).ok().map(|x| x.bottle);
    let unrepeated: Vec<&(usize, Bottle)> = bottles.into_iter().take_while(|(_, x)| Some(x.id) != last_bottle).collect();

    for (i, bottle) in unrepeated.into_iter().rev() {

        let msg = render_bottle(&bottle, *i, bottlechannelid, cfg)?;
        MakeGuildBottle {bottle: bottle.id, guild: guild.id, message: msg.id.as_i64(), time_recieved: now()}.make(conn)?;
    }

    Ok (())
}

const DELIVERNUM: i64 = 3;
pub fn distribute_bottle (bottle: &Bottle, conn:&Conn, cfg:&Config) -> Res<()> {
    let bottles: Vec<(usize, Bottle)> = bottle.get_reply_list(conn)?.into_iter().rev().enumerate().rev().collect();

    let mut query = guild::table.into_boxed();
    if let Some(guild) = bottle.guild {
        query = query.filter(guild::id.ne(guild))
    }

    let guilds: Vec<Guild> = query.filter(guild::bottle_channel.is_not_null()).order(random).limit(DELIVERNUM).load(conn)?;

    for guild in guilds {
        let _ = distribute_to_guild(&bottles, &guild, conn, cfg);
    }

    Ok(())
}

pub fn report_bottle(bottle: &Bottle, user: model::UserId, conn: &Conn, cfg: &Config) -> Res<Message> {
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

pub fn react(conn: &Conn, r: Reaction, add: bool, cfg: &Config) -> Res<()> {
    let mid = r.message_id.as_i64();
    let emojiid = match r.emoji {
        ReactionType::Custom {id: emojiid, ..} => *emojiid.as_u64(),
        _ => return Ok (())
    };

    let user = User::get(r.user_id.as_i64(), conn);

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

pub fn give_xp(bottle: &Bottle, xp: i32, conn:&Conn) -> Res<()> {
    let mut u = User::get(bottle.user, conn);
    u.xp += xp;
    u.update(conn)?;

    if let Some(g) = bottle.guild {
        let mut contribution = GuildContribution::get((g, u.id), conn);
        contribution.xp += xp;
        contribution.update(conn)?;
    }

    Ok(())
}

pub fn new_bottle(new_message: &Message, guild: Option<model::GuildId>, connpool:ConnPool, cfg:Config) -> Res<Option<String>> {
    let userid = new_message.author.id.as_i64();
    let msgid = new_message.id.as_i64();
    let conn = &connpool.get_conn();

    let mut user = User::get(userid, conn);

    let lastbottle = user.get_bottle(conn).ok();
    let ticket_res = |mut user: User, err: String| {
        user.tickets += 1;
        user.update(conn)?;

        if user.tickets > MAX_TICKETS {
            Ok(None)
        } else {
            Ok(Some(err))
        }
    };

    if let Some (ref bottle) = lastbottle {
        let since_push = now().signed_duration_since(bottle.time_pushed);
        let cooldown = Duration::minutes(COOLDOWN);

        if since_push < cooldown && !user.admin {
            let towait = cooldown - since_push;
            return ticket_res(user, format!("You must wait {} minutes before sending another bottle!", towait.num_minutes()));
        }
    }

    if !user.admin && user.get_banned(conn)? {
        return ticket_res(user, "You are banned from using Bottle! Appeal by dming the global admins!".to_owned());
    }

    let mut contents = new_message.content.clone();
    let reply_to = match guild {
        Some(g) if (&contents).starts_with(REPLY_PREFIX) => {
            contents = contents.chars().skip(REPLY_PREFIX.chars().count()).collect();
            let gbottle = Guild::get(g, conn).get_last_bottle(conn).map_err(|_| "No bottle to reply found!")?;
            Some(gbottle.bottle)
        }, _ => None
    };

    contents = contents.trim().to_owned();

    let url = new_message.embeds.get(0).and_then(|emb: &Embed| emb.url.clone());
    let image = new_message.attachments.get(0).map(|a: &Attachment| a.url.clone());

    if url.is_none() && image.is_none() && contents.len() < MIN_CHARS && !user.admin {
        return ticket_res(user, "Your bottle cannot be less than 10 characters!".to_owned());
    }

    user.tickets = 0;
    user.update(conn)?;

    let mut xp = 0;

    if let Some(bid) = reply_to {
        let replied = Bottle::get(bid, conn)?;
        if replied.user != user.id {
            give_xp(&replied, REPLYXP, conn)?;
        }
    }

    xp += PUSHXP;
    if url.is_some() { xp += URLXP; }
    if image.is_some() { xp += IMAGEXP; }

    let bottle = MakeBottle { message: msgid, reply_to, guild, user: user.id, time_pushed: now(), contents, url, image }
        .make(conn)?;

    give_xp(&bottle, xp, conn)?;

    thread::spawn(move || {
        distribute_bottle(&bottle, &connpool.get_conn(), &cfg).ok();
    });

    Ok(Some("Your message has been ~~discarded~~ pushed into the dark seas of discord!".to_owned()))
}