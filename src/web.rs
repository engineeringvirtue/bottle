use oauth2;
use nickel;
use nickel::{Nickel, StaticFilesHandler, Request, Response, MiddlewareResult, Action, NickelError, FaviconHandler, HttpRouter};
use nickel::extensions::{Redirect};
use nickel::hyper::net::Fresh;
use nickel::status::StatusCode;
use std::io::Write;

use std::collections::HashMap;

use model::*;
use data::*;

pub fn start_serv (db: ConnPool, cfg: Config) {
    let oauthcfg = oauth2::Config::new(
        cfg.client_id, cfg.client_secret, "https://discordapp.com/api/oauth2/authorize", "https://discordapp.com/api/oauth2/token"
    ).add_scope("identify").set_redirect_url("http://www.google.com").set_state("dogedoge");

    let mut serv = Nickel::new();

    serv.utilize(FaviconHandler::new("./assets/icon_transparent.png"));
    serv.utilize(StaticFilesHandler::new("./res/img"));
    serv.utilize(StaticFilesHandler::new("./res/style"));

//    serv.get("/user/:uid", |req, res| {
//        let uid = req.param("uid")?;
//
//        let conn = &db.get_conn();
//        let u = User::get(uid, conn);
//        let bottles = u.get_last_bottles(5, conn);
//
//
//    });

    serv.get("/", middleware! { |req, res|
        let conn:&Conn = &db.get_conn();

        let mut data = HashMap::new();
        data.insert("bottlecount", get_bottle_count(&conn).unwrap());
        data.insert("usercount", get_user_count(&conn).unwrap());
        data.insert("guildcount", get_guild_count(&conn).unwrap());

        return res.render("res/home.html", &data);
    });

    serv.get("/**", middleware! { |req, res|
        return res.render("res/404.html", &HashMap::<String, String>::new());
    });

    serv.listen("127.0.0.1:8080").unwrap();
}