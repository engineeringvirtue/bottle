use std::thread;
use chrono::{DateTime, Utc};
use serenity::model::id::{ChannelId, UserId, GuildId, MessageId};
use serenity::model::channel::{Message, ReactionType, Reaction, Embed, Attachment};
use time::Duration;
use diesel::prelude::*;
use serenity::utils::Colour;

use model::id::*;
use model;
use model::*;
use diesel::sql_types::BigInt;

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

pub fn render_bottle (bottle: &Bottle, edit: Option<MessageId>, mut level: usize, in_reply: bool, channel: ChannelId, cfg:&Config) -> Res<Message> {
    channel.broadcast_typing()?;

    if in_reply {
        level += 1;
    }

    let embd: Res<serenity::builder::CreateEmbed> = (|| {
        let e = serenity::builder::CreateEmbed::default();

        if bottle.deleted {
            return Ok(e.title(format!("BOTTLE FROM {} IS DELETED", get_user_name(bottle.user))).description("This bottle has been deleted."));
        }

        let title = if level > 0 { "You have found a message glued to the bottle!" } else { "You have recovered a bottle!" }; //TODO: better reply system, takes last bottle as an argument

        let mut extra_info = String::new();
        if let Some(x) = &bottle.url {
            if bottle.contents.is_empty() {
                extra_info.push_str(&format!(" [Link]({})", x));
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

        Ok(e)
    })();

    let embd = embd?;

    let msg = {
        if let Some(x) = edit {
            channel.edit_message(x, |x| x.embed(|_| embd))
        } else {
            channel.send_message(|x| x.embed(|_| embd))
        }
    }?;

    Ok(msg)
}

const DELIVERNUM: i64 = 4;

pub fn distribute_to_channel((bottles, in_reply): (&Vec<(usize, Bottle)>, &bool), channel: i64, conn: &Conn, cfg:&Config) -> Res<()> {
    let bottlechannelid = ChannelId(channel as u64);

    let last_bottle = ReceivedBottle::get_last(channel, conn).ok().map(|x| x.bottle);
    let unrepeated: Vec<&(usize, Bottle)> = bottles.into_iter().take_while(|(_, x)| Some(x.id) != last_bottle).collect();

    for (i, bottle) in unrepeated.into_iter().rev() {
        let msg = render_bottle(&bottle, None, *i, *in_reply, bottlechannelid, cfg)?;
        MakeReceivedBottle {bottle: bottle.id, channel: bottlechannelid.as_i64(), message: msg.id.as_i64(), time_recieved: now()}.make(conn)?;
    }

    trace!("Delivered bottle to channel {}", &channel);
    Ok (())
}

#[derive(QueryableByName)]
struct GuildsResult {
    #[sql_type="BigInt"] #[column_name="id"]
    id: i64,
    #[sql_type="BigInt"] #[column_name="bottle_channel"]
    bottle_channel: i64
}

pub fn distribute_bottle (bottle: &Bottle, conn:&Conn, cfg:&Config) -> Res<()> {
    let (bottles, in_reply) = bottle.get_reply_list(conn)?;
    let bottles: Vec<(usize, Bottle)> = bottles.into_iter().rev().enumerate().rev().collect();

    let guilds: Vec<GuildsResult> = diesel::sql_query(
        "SELECT \"id\", bottle_channel FROM (SELECT DISTINCT ON (guild.id) guild.id, bottle_channel, time_recieved FROM guild LEFT JOIN received_bottle ON (bottle_channel = received_bottle.channel) ORDER BY guild.id, received_bottle.time_recieved DESC) channels
        WHERE bottle_channel IS NOT NULL ORDER BY time_recieved ASC NULLS FIRST LIMIT $2")
        .bind::<BigInt, _>(bottle.channel).bind::<BigInt, _>(DELIVERNUM).load(conn)?;

    let mut channels: Vec<(Option<i64>, i64)> =
        guilds.into_iter().map(|GuildsResult {id, bottle_channel}| (Some(id), bottle_channel)).collect(); //tuple of guild and channel
    channels.extend(bottles.iter().map(|(_, b)| (None, b.channel)));
    channels.dedup();

    for (guild, channel) in channels {
        if channel != bottle.channel {
            if let Err(err) = distribute_to_channel((&bottles, &in_reply), channel, conn, cfg) {
                if let Some(guild) = guild {
                    debug!("Deleting guild {}, error sending: {}", guild, err);
                    Guild::del(guild, conn)?;
                }
            }
        }
    }

    Ok(())
}

pub fn report_bottle(bottle: &Bottle, user: model::UserId, conn: &Conn, cfg: &Config) -> Res<ReceivedBottleId> {
    let channel = ChannelId(cfg.admin_channel as u64);
    let user = UserId(user as u64).to_user()?;
    let msg = channel.say(&format!("REPORT FROM {}. USER ID {}, BOTTLE ID {}.", user.tag(), user.id, bottle.id))?;

    let bottlemsg: Message = render_bottle(&bottle, None, 0, true, channel, cfg)?;

    msg.react(cfg.ban_emoji.as_str())?;
    bottlemsg.react(cfg.ban_emoji.as_str())?;
    bottlemsg.react(cfg.delete_emoji.as_str())?;

    let recv = MakeReceivedBottle {bottle: bottle.id, channel: channel.as_i64(), message: bottlemsg.id.as_i64(), time_recieved: now()}.make(conn)?;

    Ok(recv.id)
}

pub fn del_bottle(mut b: Bottle, conn: &Conn, cfg: &Config) -> Res<()> {
    trace!("Bottle deleted");

    Bottle::del(b.id, conn)?;
    b.deleted = true;

    for rb in ReceivedBottle::get_from_bottle(b.id, conn)? {
        let _ = render_bottle(&b, Some(MessageId(rb.message as u64)), 0, false, ChannelId(rb.channel as u64), cfg);
    }

    Ok(())
}

pub fn react(conn: &Conn, r: Reaction, add: bool, cfg: &Config) -> Res<()> {
    trace!("Reaction added: {}", r.emoji.to_string());

    let user = {
        let x = r.user()?;
        if x.bot {
            return Ok(());
        }

        User::get(x.id.as_i64(), conn)
    };

    let mid = r.message_id.as_i64();

    let emoji_name = match r.emoji {
        ReactionType::Unicode(x) => x,
        _ => return Ok (())
    };

    let ban =
        |report: Report, user: model::UserId, conn: &Conn| -> Res<()> { //either received or original
            let b = Ban { user, report: Some(report.bottle) };

            if add {
                let u = User::get(user, conn);
                for x in u.get_all_bottles(conn)? {
                    del_bottle(x, conn, cfg)?;
                }

                b.make(conn)?;
            } else {
                b.del(conn)?;
            }

            Ok(())
        };

    if user.admin {
        let user = user.id;

        if emoji_name == cfg.ban_emoji {
            if let Ok(recv) = ReceivedBottle::get_from_message(mid, conn) {
                let buser = Bottle::get(recv.bottle, conn)?.user;

                if let Ok(report) = Report::get_from_recv_user(recv.id, user, conn) { ban(report, buser, conn)?; } else {
                    let rep = Report { bottle: recv.bottle, user, received_bottle: Some(recv.id) }.make(conn)?;
                    ban(rep, buser, conn)?;
                }
            } else if let Ok(bottle) = Bottle::get_from_message(mid, conn) {
                let rep = Report { bottle: bottle.id, user, received_bottle: None }.make(conn)?;
                ban( rep, bottle.user, conn)?;
            }
        } else if let Ok(bottle) = Bottle::get_recv_or_bottle_from_message(mid, conn) {
            if emoji_name == cfg.delete_emoji && add {
                del_bottle(bottle, conn, cfg)?;
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

pub fn bottle_from_msg(message: &Message, edit: bool, guild: Option<model::GuildId>, conn:&Conn, _cfg:&Config) -> Res<Result<MakeBottle, Option<String>>> { //kill meh naw
    let userid = message.author.id.as_i64();
    let msgid = message.id.as_i64();
    let channelid = message.channel_id.as_i64();

    let mut user = User::get(userid, conn);

    let lastbottle = user.get_bottle(conn).ok();
    let ticket_res = |mut user: User, err: String| {
        user.tickets += 1;
        user.update(conn)?;

        if user.tickets > MAX_TICKETS {
            Ok(Err(None))
        } else {
            Ok(Err(Some(err)))
        }
    };

    if !user.admin && !edit {
        if user.get_banned(conn)? {
            return ticket_res(user, "You are banned from using Bottle! Appeal by dming the global admins!".to_owned());
        }

        if let Some(ref bottle) = lastbottle {
            let since_push = now().signed_duration_since(bottle.time_pushed);
            let cooldown = Duration::minutes(COOLDOWN);

            if since_push < cooldown {
                let towait = cooldown - since_push;
                return ticket_res(user, format!("You must wait {} seconds before sending another bottle!", towait.num_seconds()));
            }
        }
    }

    let mut contents = message.content.clone();

    let get_reply_to = || -> Res<Option<i64>> {
        let rbottle = ReceivedBottle::get_last(channelid, conn).map_err(|_| "No bottle to reply to found!")?;
        Ok(Some(rbottle.bottle))
    };

    let reply_to =
        if (&contents).starts_with(REPLY_PREFIX) {
            contents.drain(..REPLY_PREFIX.len());
            get_reply_to()?
        } else if (&contents).starts_with(ALT_REPLY_PREFIX) {
            contents.drain(..ALT_REPLY_PREFIX.len());
            get_reply_to()?
        } else {
            None
        };

    contents = contents.trim().to_owned();

    let url = message.embeds.get(0).and_then(|emb: &Embed| emb.url.clone());
    let image = message.attachments.get(0).map(|a: &Attachment| a.url.clone());

    if url.is_none() && image.is_none() && contents.len() < MIN_CHARS && !user.admin {
        return ticket_res(user, "Your bottle cannot be less than 10 characters!".to_owned());
    }

    user.tickets = 0;
    user.update(conn)?;

    Ok(Ok(MakeBottle { message: msgid, reply_to, channel: channelid, guild, user: user.id, time_pushed: now(), contents, url, image }))
}

pub fn new_bottle(new_msg: &Message, guild: Option<model::GuildId>, connpool:ConnPool, cfg:Config) -> Res<Option<String>> {
    trace!("New bottle found");
    let conn = &connpool.get_conn();

    let bottle =
        match bottle_from_msg(new_msg, false, guild, conn, &cfg)? {
            Ok(x) => x,
            Err(x) => return Ok(x)
        };

    let bottle = bottle.make(conn)?;

    let mut xp = 0;

    xp += PUSHXP;

    if let Some(r) = bottle.reply_to {
        let replied = Bottle::get(r, conn)?;
        if replied.user != new_msg.author.id.as_i64() {
            give_xp(&replied, REPLYXP, conn)?;
        }
    }

    if bottle.url.is_some() { xp += URLXP; }
    if bottle.image.is_some() { xp += IMAGEXP; }

    give_xp(&bottle, xp, conn)?;

    debug!("Sending bottle: {:?}", &bottle);

    thread::spawn(move || {
        let _ = distribute_bottle(&bottle, &connpool.get_conn(), &cfg);
    });

    Ok(Some("Your message has been ~~discarded~~ pushed into the dark seas of discord!".to_owned()))
}