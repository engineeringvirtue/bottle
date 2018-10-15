use model::*;

use diesel::prelude::*;
use std::error::Error;
use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};
use diesel::pg::PgConnection;
use schema::*;
use diesel::*;
use diesel::dsl::*;

type Res<a> = Result<a, result::Error>;
pub trait DataAccess {  
    // fn get_user(User)

    // fn update_user(User, closure)

    // fn get_user_last_bottle(User)


    // fn get_guild(Guild)

    // fn update_guild

    // fn del_guild


    // fn make_bottle

    // fn get_bottle

    // fn get_random_uncollected_bottle


    // fn make_bottle_user

    // fn get_bottle_user

    // fn del_bottle_user


    // fn make_report

    // fn get_report

    // fn del_report


    fn get_bottle_count (&self) -> Res<i64>;
    fn get_user_count (&self) -> Res<i64>;
    fn get_guild_count (&self) -> Res<i64>;
}

impl DataAccess for Conn {
    fn get_bottle_count (&self) -> Res<i64> {
        bottle::table.select(count_star()).first(self)
    }

    fn get_user_count (&self) -> Res<i64> {
        user::table.select(count_star()).first(self)
    }

    fn get_guild_count (&self) -> Res<i64> {
        guild::table.select(count_star()).first(self)
    }
}