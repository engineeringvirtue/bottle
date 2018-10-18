use model::*;

use diesel::prelude::*;
use schema::*;
use diesel::*;

type Res<A> = Result<A, result::Error>;

pub mod functions {
    use diesel::sql_types::*;
    use diesel::query_source::*;
    use diesel::query_builder::*;
    use diesel::result::QueryResult;
    use diesel::pg::Pg;

    pub struct Ranking<A, B> {
        compare_col: A,
        source: B
    }

    impl<A, B> QueryFragment<Pg> for Ranking<A,B> where A: QueryFragment<Pg>, B: QueryFragment<Pg> {
        fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
//            SELECT COUNT(*)
//            FROM "user" as compare
//            WHERE compare.xp > "user".xp
//            ) AS Ranking

            out.push_sql("1 + (SELECT COUNT(*) FROM ");
            self.source.walk_ast(out.reborrow())?;
            out.push_sql(" AS compare WHERE compare.");
            self.compare_col.walk_ast(out.reborrow())?;
            out.push_sql(" > ");
            self.source.walk_ast(out.reborrow())?;
            out.push_sql(".");
            self.compare_col.walk_ast(out.reborrow())?;
            out.push_sql(") AS Ranking");

            Ok(())
        }
    }

    impl_query_id!(Ranking<A, B>);

    impl<A, B> diesel::expression::Expression for Ranking<A, B> {
        type SqlType = BigInt;
    }

    impl_selectable_expression!(Ranking<A, B>);

    impl<A, B> Ranking<A, B> where A: QueryFragment<Pg>, B: QueryFragment<Pg> {
        pub fn new<C, D> (compare_col: C, source: D) -> Ranking<C::Query, D::Query> where C: AsQuery + Sized, D: AsQuery + Sized {
            Ranking { compare_col: compare_col.as_query(), source: source.as_query() }
        }
    }

    no_arg_sql_function!(random, (), "Represents the postgresql random() function");
    sql_function!(estimate_rows, Estimate, (tablename: Text) -> Int8);
}

use self::functions::*;

impl User {
    pub fn get(uid: UserId, conn:&Conn) -> Self {
        user::table.find(uid).first(conn).unwrap_or_else(|_| User::new(uid))
    }

    pub fn update(&self, conn:&Conn) -> Res<usize> {
        insert_into(user::table).values(self).on_conflict(user::id).do_update().set(self).execute(conn)
    }

    pub fn get_last_bottles(&self, limit:i64, conn:&Conn) -> Res<Vec<Bottle>> {
        Bottle::belonging_to(self).order(bottle::time_pushed.desc()).limit(limit).load(conn)
    }

    pub fn get_ranking(&self, conn:&Conn) -> Res<i64> {
        user::table.select(Ranking::new(user::xp, user::table)).find(self.id).first(conn)
    }
}
//
impl Guild {
    pub fn get(gid: GuildId, conn:&Conn) -> Self {
        guild::table.find(gid).first(conn).unwrap_or_else(|_| Guild::new(gid))
    }

    pub fn update(&self, conn:&Conn) -> Res<usize> {
        insert_into(guild::table).values(self).on_conflict(guild::id).do_update().set(self).execute(conn)
    }

    pub fn get_last_bottle(&self, conn:&Conn) -> Res<GuildBottle> {
        GuildBottle::belonging_to(self).order(guild_bottle::time_recieved.desc()).first(conn)
    }

    pub fn delete(gid: GuildId, conn:&Conn) -> Res<usize> {
        delete(guild::table).filter(guild::id.eq(gid)).execute(conn)
    }
}
//
impl MakeBottle {
    pub fn make(&self, conn:&Conn) -> Res<Bottle> {
        insert_into(bottle::table).values(self).get_result(conn)
    }
}

impl Bottle {
    pub fn get(id:BottleId, conn:&Conn) -> Res<Self> {
        bottle::table.find(id).get_result(conn)
    }

    pub fn delete(id:BottleId, conn:&Conn) -> Res<usize> {
        delete(bottle::table).filter(bottle::id.eq(id)).execute(conn)
    }

    pub fn get_reply_list(&self, conn:&Conn) -> Res<Vec<Self>> {
        let mut bottles: Vec<Bottle> = Vec::new();

        while bottles.len() < 25 {
            match bottles.last().unwrap_or(self).reply_to {
                Some(x) => {
                    bottles.push(Bottle::get(x, conn)?);
                },
                None => break
            }
        }

        Ok(bottles)
    }
}

impl MakeGuildBottle {
    pub fn make(&self, conn:&Conn) -> Res<GuildBottle> {
        insert_into(guild_bottle::table).values(self).get_result(conn)
    }
}

impl GuildBottle {
    pub fn get(buid:GuildBottleId, conn:&Conn) -> Res<Self> {
        guild_bottle::table.find(buid).get_result(conn)
    }

    pub fn get_from_message(mid:i64, conn:&Conn) -> Res<Self> {
        guild_bottle::table.filter(guild_bottle::message.eq(mid)).get_result(conn)
    }

    pub fn delete(buid:GuildBottleId, conn:&Conn) -> Res<usize> {
        delete(guild_bottle::table.find(buid)).execute(conn)
    }
}

//impl Report {
//    fn make(&self, conn:&Conn) -> Res<ReportId> {
//
//    }
//
//    fn get_from_message(mid:i64, conn:&Conn) -> Res<(ReportId, Self)> {
//
//    }
//
//    fn del(rid:ReportId) -> Res<()> {
//
//    }
//}

pub fn get_bottle_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("bottle".to_owned())).get_result(conn)
}

pub fn get_user_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("user".to_owned())).get_result(conn)
}

pub fn get_guild_count (conn: &Conn) -> Res<i64> {
    select(estimate_rows("guild".to_owned())).get_result(conn)
}