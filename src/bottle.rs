use std::thread;
use chrono::{DateTime, Utc};
use serenity::prelude::*;
use serenity::model::id::{ChannelId, UserId, GuildId};
use serenity::model::channel::Message;
use serenity::CACHE;
use diesel::prelude::*;
use diesel::expression::Expression;
use serenity::utils::Colour;
use diesel::sql_types;

use model::id::*;
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

pub fn render_bottle (bottle: &Bottle, level: usize, channel: ChannelId) -> Res<Message> {
    let msg = channel.send_message(|x| x.embed(|e| {
        let title = if level > 0 { "You have found a message glued to the bottle!" } else { "You have recovered a bottle!" }; //TODO: better reply system, takes last bottle as an argument

        let mut e = e.title(title)
            .description(bottle.contents.clone())
            .timestamp(&DateTime::<Utc>::from_utc(bottle.time_pushed, Utc))
            .color(level_to_col(level))
            .footer(|footer|
                if let Some(ref guild) = bottle.guild.and_then(|guild| GuildId(guild as u64).to_partial_guild().ok()) {
                    let mut f = footer.text(&guild.name);
                    if let Some(ref icon) = guild.icon {
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

                    author.name(&username).icon_url(&avatar)
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

pub fn distribute_to_guild(bottles: &Vec<Bottle>, guild: Guild, conn: &Conn) -> Res<()> {
    let bottlechannelid = ChannelId(guild.bottle_channel.ok_or("No bottle channel")? as u64);

    for (i, bottle) in bottles.iter().rev().enumerate() {
        let msg = render_bottle(bottle, i, bottlechannelid)?;
        MakeGuildBottle {bottle: bottle.id, guild: guild.id, message: msg.id.as_i64(), time_recieved: now()}.make(conn)?;
    }

    Ok (())
}

pub fn distribute_bottle (bottle: MakeBottle, conn:&Conn) -> Res<()> {
    let bottle = bottle.make(conn)?;

    let mut query = guild::table.into_boxed();

    if let Some(guild) = bottle.guild {
        query = query.filter(guild::id.ne(guild))
    }

    query.filter(guild::bottle_channel.is_not_null()).order(random).first(conn)
        .map_err(|err| -> Box<Error> { err.into() })
        .and_then(|guild: Guild| -> Res<()> {
            let mut bottles = bottle.get_reply_list(conn)?;
            bottles.insert(0, bottle);

            distribute_to_guild(&bottles, guild, conn)?;

            Ok(())
        }).ok();

    Ok(())
}