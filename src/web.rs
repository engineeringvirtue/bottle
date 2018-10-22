use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use oauth2;
use iron::prelude::*;
use iron::{BeforeMiddleware, AfterMiddleware, AroundMiddleware, status};
use iron_sessionstorage_0_6::traits::*;
use iron_sessionstorage_0_6::{Session, SessionStorage, backends::SignedCookieBackend};
use handlebars_iron::{Template, handlebars::Context, HandlebarsEngine, DirectorySource};
use router::Router;
use staticfile::Static;
use mount::Mount;
use params::{Params, Value};
use serenity::model::id;
use serde_json;

use model::*;
use data::*;

#[derive(Clone, Deserialize, Serialize)]
struct SessionData {
    session_id: Uuid,
    redirect: Option<String>
}

impl SessionData {
    fn new() -> Self {
        SessionData {session_id: Uuid::new_v4(), redirect: None}
    }
}

impl iron_sessionstorage_0_6::Value for SessionData {
    fn get_key() -> &'static str { "bd_session" }
    fn into_raw(self) -> String { serde_json::to_string(&self).unwrap() }
    fn from_raw(value: String) -> Option<Self> {
        serde_json::from_str(&value).ok()
    }
}

struct PrerequisiteMiddleware {pool: ConnPool, oauth: oauth2::Config}

impl BeforeMiddleware for PrerequisiteMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DConn>(self.pool.clone());
        req.extensions.insert::<DOauth2>(self.oauth.clone());
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
                Ok(Response::with(Template::new("404", 0)))
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

    let resp = Response::with(Template::new("home", &data));
    Ok(resp)
}

fn user(req: &mut Request) -> IronResult<Response> {
    let conn: &Conn = &req.get_conn();
    let uid = req.extensions.get::<Router>().unwrap()
        .find("user").and_then(|x| x.parse().ok()).unwrap();

    let udata = get_user_data(uid, conn).unwrap();

    let resp = Response::with(Template::new("user", &udata));
    Ok(resp)
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct DUserData {
    id: i64
}

const GETUSER: &str = "https://discordapp.com/users/@me";
impl DUserData {
    fn get(access_token: String) -> Res<Self> {
        use http_req::{request::RequestBuilder, url::Url};

        let mut body = HashMap::new();
        body.insert("access_token", access_token);

        let b = bincode::serialize(&body)?;
        let res = RequestBuilder::new(Url::from_str(GETUSER)?)
            .body(b.as_slice()).send()?;

        Ok(bincode::deserialize(res.body())?)
    }
}

fn set_session(sesd: SessionData, ses: &mut Session) {
   ses.set(sesd).unwrap()
}

fn get_session(ses: &Session) -> SessionData {
    ses.get::<SessionData>().unwrap().unwrap_or(SessionData::new())
}

fn get_user(ses: SessionData, conn: &Conn) -> Option<User> {
    User::from_ses(ses.session_id, conn).ok()
}

//o look an oauth token, lez jus pass it in here
fn set_tok(ses: &mut Session, tok: oauth2::Token, conn: &Conn) -> Res<()> {
    let uid = DUserData::get(tok.access_token)?.id;
    let mut u = User::get(uid, conn);

    let sesd = get_session(ses);
    let session_id = sesd.session_id;
    u.session = Some(sesd.session_id);
    u.update(conn)?;

    Ok(())
}

//report: get_user -> make report... if get user fails, redirect to oauth with hashed session id and set redirect url
//redirect: check code, set token, redirect to redirect url

pub fn start_serv (db: ConnPool, cfg: Config) {
    let oauthcfg = oauth2::Config::new(
        cfg.client_id, cfg.client_secret, "https://discordapp.com/api/oauth2/authorize", "https://discordapp.com/api/oauth2/token"
    )
        .add_scope("identify")
        .set_redirect_url(format!("{}/oauth", cfg.host_url));

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/:user", user, "user");
//    router.get("/report/:bottle", report, "report");
//    router.get("/oauth", redirect, "redirect");

    let mut chain = Chain::new(router);

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./res/", ".hbs")));
    hbse.reload().unwrap();

    chain.link_before(PrerequisiteMiddleware {pool: db, oauth: oauthcfg});
    chain.link_after(hbse);
    chain.link_after(StatusMiddleware);
    chain.link_around(SessionStorage::new(SignedCookieBackend::new(cfg.cookie_sig.clone().into_bytes())));

    let mut mount = Mount::new();
    mount.mount("/", chain);
    mount.mount("/style", Static::new("./res/style"));
    mount.mount("/img", Static::new("./res/img"));

    let iron = Iron::new(mount);
    let web = iron.http(&cfg.host_url).unwrap();
}