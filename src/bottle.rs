use chrono::{DateTime, Utc};
use serenity::prelude::*;
use serenity::model::id::*;
use diesel::prelude::*;

use model::id::*;
use model::*;
use data::*;

pub fn distribute_bottle (bottle: MakeBottle, ctx: &Context, conn:&Conn) -> Res<()> {
    let bottle = bottle.make(conn)?;
    let guilddata = Guild::get_random(conn)?;
    let bottlechannelid = ChannelId(guilddata.bottle_channel.ok_or("No bottle channel")? as u64);

    let msg = bottlechannelid.send_message(|x| x.embed(|e|
        e.title("You have recovered a bottle!").description(bottle.contents.clone()).timestamp(&DateTime::<Utc>::from_utc(bottle.time_pushed.clone(), Utc))
    ))?;

    MakeGuildBottle {bottle: bottle.id, guild: guilddata.id, message: msg.id.as_i64()}.make(conn)?;
    Ok (())
}