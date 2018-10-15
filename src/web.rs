use oauth2;
use r2d2::Pool;
use std::sync::{Arc};
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use nickel::{Nickel, StaticFilesHandler, HttpRouter};
use nickel::extensions::{Redirect};

use std::collections::HashMap;

use model::*;
use data::*;

pub fn start_serv (db: ConnPool, cfg: Config) {
    let mut oauthcfg = oauth2::Config::new(
        cfg.client_id, cfg.client_secret, "https://discordapp.com/api/oauth2/authorize", "https://discordapp.com/api/oauth2/token"
    ).add_scope("identify").set_redirect_url("http://www.google.com").set_state("dogedoge");

    let mut serv = Nickel::new();

    serv.utilize(StaticFilesHandler::new("/res"));

    serv.get("/auth/redirect", middleware! { |req, res|
        "wuht"
    });

    serv.get("/auth", middleware! { |req, res|
        return res.redirect(oauthcfg.authorize_url().into_string());
    });

    serv.get("/", middleware! { |req, res|
        let conn:Conn = db.get().unwrap();

        let mut data = HashMap::new();
        data.insert("bottlecount", get_bottle_count(&conn).unwrap());
        data.insert("usercount", get_user_count(&conn).unwrap());
        data.insert("guildcount", get_guild_count(&conn).unwrap());

        return res.render("res/home.tpl", &data);
    });

    serv.listen("127.0.0.1:8080").unwrap();
}