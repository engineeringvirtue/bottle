use oauth2;
use r2d2::Pool;
use std::sync::{Arc};
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use nickel::{Nickel, StaticFilesHandler, HttpRouter};
use nickel::extensions::{Redirect};

use std::collections::HashMap;

use model::*;
use data::DataAccess;

pub fn start_serv (db: ConnPool, cfg: Arc<Config>) {
    let mut oauthcfg = oauth2::Config::new(
        cfg.client_id, cfg.client_secret, "https://discordapp.com/api/oauth2/authorize", "https://discordapp.com/api/oauth2/token"
    ).add_scope("identify").set_redirect_url("http://www.google.com").set_state("dogedoge");

    let mut serv = Nickel::new();
    
    serv.get("/auth/redirect", middleware! { |req, res|
        return res.redirect(oauthcfg.authorize_url().into_string());
    });

    serv.get("/auth", middleware! { |req, res|
        println!("goin 2 auth");
        return res.redirect(oauthcfg.authorize_url().into_string());
    });

    serv.get("/", middleware! { |req, res|
        let conn:Conn = db.get().unwrap();

        let mut data = HashMap::new();
        data.insert("bottlecount", conn.get_bottle_count().unwrap());
        data.insert("usercount", conn.get_user_count().unwrap());
        data.insert("guildcount", conn.get_guild_count().unwrap());

        return res.render("res/home.tpl", &data);
    });



    serv.utilize(StaticFilesHandler::new("/res"));
    serv.listen("127.0.0.1:8080").unwrap();
}