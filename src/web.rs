use oauth2;
use serenity::model::id;
use std::io::Write;

use iron::prelude::*;
use iron::{BeforeMiddleware, AfterMiddleware, status};
use mount::Mount;
use router::{Router, Params};
use staticfile::Static;
use handlebars_iron::{Template, HandlebarsEngine, DirectorySource};
use std::collections::HashMap;

use model::*;
use data::*;

struct ConnectionMiddleware {pool: ConnPool}

impl BeforeMiddleware for ConnectionMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DConn>(self.pool.clone());
        Ok(())
    }
}

impl<'a, 'b> GetConnection for Request<'a, 'b> {
    fn get_pool(&self) -> ConnPool {
        self.extensions.get::<DConn>().unwrap().get_pool()
    }
}

struct StatusMiddleware;
impl AfterMiddleware for StatusMiddleware {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        match res.status {
            Some(status::NotFound) => {
                let mut res = Response::new();
                res.set_mut(Template::new("404", 0));
                Ok(res)
            },
            _ => Ok(res)
        }
    }

    fn catch(&self, req: &mut Request, err: IronError) -> IronResult<Response> {
        self.after(req, err.response)
    }
}

fn get_user_data(uid: UserId, conn: &Conn) -> Res<HashMap<&str, String>> {
    let udata = User::get(uid, conn);
    let bottles = udata.get_last_bottles(5, conn)?;
    let user = id::UserId(udata.id as u64).to_user()?;
    let mut data = HashMap::new();
    data.insert("tag", user.tag());
    data.insert("pfp", user.avatar_url().unwrap_or(ANONYMOUS_AVATAR.to_owned()));
    data.insert("xp", udata.xp.to_string());
    data.insert("ranked", udata.get_ranking(conn)?.to_string());
    data.insert("numbottles", udata.get_num_bottles(conn)?.to_string());

    Ok(data)
}

fn home(req: &mut Request) -> IronResult<Response> {
    let conn: &Conn = &req.get_conn();

    let mut data = HashMap::new();
    data.insert("bottlecount", get_bottle_count(&conn).unwrap());
    data.insert("usercount", get_user_count(&conn).unwrap());
    data.insert("guildcount", get_guild_count(&conn).unwrap());

    let resp = Response::new().set(Template::new("home", &data));
    Ok(resp)
}

fn user(req: &mut Request) -> IronResult<Response> {
    let conn: &Conn = &req.get_conn();
    let uid = req.extensions.get::<Router>().unwrap().find("uid").ok_or("No user id provided!")?.parse()?;
    let udata = get_user_data(uid, conn).unwrap();

    let resp = Response::new().set(Template::new("user", &udata));
    Ok(resp)
}

pub fn start_serv (db: ConnPool, cfg: Config) {
    let oauthcfg = oauth2::Config::new(
        cfg.client_id, cfg.client_secret, "https://discordapp.com/api/oauth2/authorize", "https://discordapp.com/api/oauth2/token"
    ).add_scope("identify").set_redirect_url("http://www.google.com").set_state("dogedoge");

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/:uid", user, "user");

    let mut chain = Chain::new(router);

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./res/", ".hbs")));
    hbse.reload().unwrap();

    chain.link_before(ConnectionMiddleware {pool: db});
    chain.link_after(hbse);
    chain.link_after(StatusMiddleware);

    let mut mount = Mount::new();
    mount.mount("/", chain);
    mount.mount("/style", Static::new("./res/style"));
    mount.mount("/img", Static::new("./res/img"));

    let mut iron = Iron::new(mount);
    let web = iron.http(&cfg.host_path).unwrap();
}