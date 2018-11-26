use std::thread;
use chrono::{DateTime, Utc};
use serenity::model::misc::EmojiIdentifier;
use serenity::model::id::{ChannelId, UserId, GuildId, EmojiId, MessageId};
use serenity::model::channel::{Message, ReactionType, Reaction, Embed, Attachment};
use time::Duration;
use diesel::prelude::*;
use serenity::utils::Colour;

use model::id::*;
use model;
use model::*;
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

        let mut extra_info = String::new();
        if let Some(x) = &bottle.url {
            if bottle.contents.is_empty() {
                extra_info.push_str(&format!("[Link]({})", x));
            }
        };

        if let Some(x) = bottle.guild {
            extra_info.push_str(&format!(" [Guild]({})", guild_url(x, cfg)))
        }

        let mut e = e.title(title)
            .description(format!("{}{} [Report]({})", bottle.contents, extra_info, report_url(bottle.id, cfg)))
            .timestamp(&DateTime::<Utc>::from_utc(bottle.time_pushed, Utc))
            .color(col_wheel(level))
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
                    let user: Result<serenity::model::user::User, serenity::Error> = UserId(bottle.user as u64).to_user();
                    let username = user.as_ref().map(|u| u.tag())
                        .unwrap_or_else(|_| "Error fetching username".to_owned());

                    let avatar = user.as_ref().ok().and_then(|u| u.avatar_url()).unwrap_or_else(|| anonymous_url(cfg));

                    author.url(&user_url(bottle.user, cfg))
                        .name(&username).icon_url(&avatar)
                } else {
                    author.name("Anonymous").icon_url(&anonymous_url(cfg))
                }
            });

        if let Some(img) = &bottle.image {
            e = e.image(img).url(img);
        }

        if let Some(url) = &bottle.url {
            e = e.url(url);
        }

        e
    }))?;

    Ok(msg)
}

pub fn distribute_to_channel(bottles: &Vec<(usize, Bottle)>, channel: i64, conn: &Conn, cfg:&Config) -> Res<()> {
    let bottlechannelid = ChannelId(channel as u64);

    let last_bottle = ReceivedBottle::get_last(channel, conn).ok().map(|x| x.bottle);
    let unrepeated: Vec<&(usize, Bottle)> = bottles.into_iter().take_while(|(_, x)| Some(x.id) != last_bottle).collect();

    for (i, bottle) in unrepeated.into_iter().rev() {

        let msg = render_bottle(&bottle, *i, bottlechannelid, cfg)?;
        MakeReceivedBottle {bottle: bottle.id, channel: bottlechannelid.as_i64(), message: msg.id.as_i64(), time_recieved: now()}.make(conn)?;
    }

    trace!("Delivered bottle to channel {}", &channel);
    Ok (())
}

const DELIVERNUM: i64 = 1;
pub fn distribute_bottle (bottle: &Bottle, conn:&Conn, cfg:&Config) -> Res<()> {
    let bottles: Vec<(usize, Bottle)> = bottle.get_reply_list(conn)?.into_iter().rev().enumerate().rev().collect();

    let mut reply_channels: Vec<i64> = bottles.iter().skip(1).map(|(_, b)| b.channel).collect();
    reply_channels.dedup();

    let mut query = guild::table.filter(guild::bottle_channel.ne(bottle.channel)).into_boxed();

    for x in reply_channels { //distribute and filter
        let _ = distribute_to_channel(&bottles, x, conn, cfg);
        query = query.filter(guild::bottle_channel.ne(x));
    }

    let guilds: Vec<Guild> = query.filter(guild::bottle_channel.is_not_null()).order(random).limit(DELIVERNUM).load(conn)?;

    for guild in guilds {
        if let Some(channel) = guild.bottle_channel {
            let _ = distribute_to_channel(&bottles, channel, conn, cfg);
        }
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

    MakeReceivedBottle {bottle: bottle.id, channel: channel.as_i64(), message: bottlemsg.id.as_i64(), time_recieved: now()}.make(conn)?;

    Ok(msg)
}

pub fn del_bottle(bid: BottleId, conn:&Conn) -> Res<()> {
    trace!("Bottle deleted");

    for b in ReceivedBottle::get_from_bottle(bid, conn)? {
        if let Some(mut msg) = ChannelId(b.channel as u64).message(MessageId(b.message as u64)).ok() {
            let _ = msg.edit(|x| x.embed(|x| x.title("DELETED").description("This bottle has been deleted.")));
        }
    }

//    Bottle::del(bid, conn)?; bwehehehehe
    Ok(())
}

pub fn react(conn: &Conn, r: Reaction, add: bool, cfg: &Config) -> Res<()> {
    trace!("Reaction added: {}", r.emoji.to_string());

    let mid = r.message_id.as_i64();
    let emojiid = match r.emoji {
        ReactionType::Custom {id: emojiid, ..} => *emojiid.as_u64(),
        _ => return Ok (())
    };

    let user = User::get(r.user_id.as_i64(), conn);

    if user.admin {
        if let Ok(bottle) = Bottle::get_from_message(mid, conn) {
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
    trace!("New bottle found");

    let userid = new_message.author.id.as_i64();
    let msgid = new_message.id.as_i64();
    let channelid = new_message.channel_id.as_i64();
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
    let reply_to =
        if (&contents).starts_with(REPLY_PREFIX) {
            contents = contents.chars().skip(REPLY_PREFIX.chars().count()).collect();
            let rbottle = ReceivedBottle::get_last(channelid, conn).map_err(|_| "No bottle to reply to found!")?;

            Some(rbottle.bottle)
        } else {None};

    contents = contents.trim().to_owned();

    let url = new_message.embeds.get(0).and_then(|emb: &Embed| emb.url.clone());
    let image = new_message.attachments.get(0).map(|a: &Attachment| a.url.clone());

    if url.is_none() && image.is_none() && contents.len() < MIN_CHARS && !user.admin {
        return ticket_res(user, "Your bottle cannot be less than 10 characters!".to_owned());
    }

    user.tickets = 0;
    user.update(conn)?;

    let mut xp = 0;

    if let Some(r) = reply_to {
        let replied = Bottle::get(r, conn)?;
        if replied.user != user.id {
            give_xp(&replied, REPLYXP, conn)?;
        }
    }

    xp += PUSHXP;
    if url.is_some() { xp += URLXP; }
    if image.is_some() { xp += IMAGEXP; }

    let bottle = MakeBottle { message: msgid, reply_to, channel: channelid, guild, user: user.id, time_pushed: now(), contents, url, image }
        .make(conn)?;

    give_xp(&bottle, xp, conn)?;
    debug!("Sending bottle: {:?}", &bottle);

    thread::spawn(move || {
        let _ = distribute_bottle(&bottle, &connpool.get_conn(), &cfg);
    });

    Ok(Some("Your message has been ~~discarded~~ pushed into the dark seas of discord!".to_owned()))
}