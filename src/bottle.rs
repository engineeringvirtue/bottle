use serenity::prelude::*;
use serenity::CACHE;
use diesel::prelude::*;
use diesel;

use model::*;
use schema::*;
use data::*;

pub fn distribute_bottle (bottle: MakeBottle, ctx: &Context, conn:&Conn) -> Res<()> {
//    bottle.make(conn);
//
//    users::table.select(users::userid)
    Ok (())
}