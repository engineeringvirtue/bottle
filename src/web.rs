use std;
use std::{collections::HashMap, fmt};
use std::str::{FromStr, from_utf8};
use uuid::Uuid;

use oauth2;
use iron;
use iron::prelude::*;
use iron::{BeforeMiddleware, AfterMiddleware, AroundMiddleware, status, modifiers::{RedirectRaw, Redirect}};
use iron_sessionstorage_0_6;
use iron_sessionstorage_0_6::traits::*;
use iron_sessionstorage_0_6::{Session, SessionStorage, backends::SignedCookieBackend};
use handlebars_iron::{Template, HandlebarsEngine, DirectorySource};
use router::{Router, NoRoute};
use staticfile::Static;
use mount::Mount;
use params::{Params, Value};
use serenity::model::id;
use serenity::model::guild;
use serde_json;

use model::*;
use model::id::*;
use data::*;
use bottle;

#[derive(Clone, Deserialize, Serialize)]
struct SessionData {
    id: Uuid,
    redirect: Option<String>
}

impl SessionData {
    fn new() -> Self {
        SessionData { id: Uuid::new_v4(), redirect: None}
    }
}

impl iron_sessionstorage_0_6::Value for SessionData {
    fn get_key() -> &'static str { "bd_session" }
    fn into_raw(self) -> String { serde_json::to_string(&self).unwrap() }
    fn from_raw(value: String) -> Option<Self> {
        serde_json::from_str(&value).ok()
    }
}

#[derive(Debug)]
struct InternalError(String);
#[derive(Debug)]
struct ParamError;
#[derive(Debug)]
struct AuthError;

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let InternalError(desc) = self;
        write!(f, "An internal error occured: {}", desc)
    }
}

impl iron::Error for InternalError {}

impl InternalError {
    fn with<T, F: Fn() -> Res<T>>(f: F) -> IronResult<T> {
        f().map_err(|err| {
            IronError::new(InternalError(err.description().to_string()), status::InternalServerError)
        })
    }
}

impl fmt::Display for ParamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error finding/parsing a parameter")
    }
}

impl iron::Error for ParamError {}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error authorizing")
    }
}

impl iron::Error for AuthError {}

struct PrerequisiteMiddleware {pool: ConnPool, oauth: oauth2::Config, cfg: Config}

impl BeforeMiddleware for PrerequisiteMiddleware {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<DConn>(self.pool.clone());
        req.extensions.insert::<DOauth2>(self.oauth.clone());
        req.extensions.insert::<DConfig>(self.cfg.clone());

        Ok(())
    }
}

impl<'a, 'b> GetConnection for Request<'a, 'b> {
    fn get_pool(&self) -> ConnPool {
        self.extensions.get::<DConn>().unwrap().get_pool()
    }
}

impl<'a, 'b> GetConfig for Request<'a, 'b> {
    fn get_cfg(&self) -> Config {
        self.extensions.get::<DConfig>().unwrap().clone()
    }
}

struct StatusMiddleware;
impl AfterMiddleware for StatusMiddleware {
    fn catch(&self, _req: &mut Request, err: IronError) -> IronResult<Response> {
        if err.error.is::<NoRoute>() || err.error.is::<ParamError>() {
            Ok(Response::with((status::NotFound, Template::new("notfound", &false))))
        } else {
            Err(err)
        }
    }
}

pub fn get_guild_name(id: GuildId) -> String {
    use serenity::model::id::GuildId;
    GuildId(id as u64).to_guild_cached().map(|x| x.read().name.to_owned())
        .unwrap_or_else(|| "Guild not found".to_owned())
}

pub fn get_user_name(id: UserId) -> String {
    use serenity::model::id::UserId;
    UserId(id as u64).to_user().ok().map(|x| x.name).unwrap_or_else(|| "User not found".to_owned())
}

#[derive(Deserialize, Serialize)]
struct BottlePage {
    contents: String, time_pushed: String, image: Option<String>, guild: Option<String>
}

#[derive(Deserialize, Serialize)]
struct UserContribution {guild: String, gid: i64, xp: i32}
#[derive(Deserialize, Serialize)]
struct UserPage {
    tag: String, admin: bool, pfp: String, xp: i32, ranked: i64, num_bottles: i64, contributions: Vec<UserContribution>, recent_bottles: Vec<BottlePage>
}

#[derive(Deserialize, Serialize)]
struct GuildContribution {user: String, uid: i64, xp: i32}
#[derive(Deserialize, Serialize)]
struct GuildPage {
    name: String, pfp: String, invite: Option<String>, xp: i64, ranked: Option<i64>, num_bottles: i64, contributions: Vec<GuildContribution>
}

fn get_user_data(uid: UserId, conn: &Conn, cfg: &Config) -> Res<UserPage> {
    let udata = User::get(uid, conn);
    let user = id::UserId(udata.id as u64).to_user()?;

    let data = UserPage {
        tag: user.tag(), admin: udata.admin,
        pfp: user.avatar_url().unwrap_or_else(|| anonymous_url(cfg)),
        xp: udata.xp,
        ranked: udata.get_ranking(conn)?,
        num_bottles: udata.get_num_bottles(conn)?,
        contributions: udata.get_contributions(5, conn)?.into_iter().map(|c| {
            UserContribution {guild: get_guild_name(c.guild), gid: c.guild, xp: c.xp}
        }).collect(),
        recent_bottles: udata.get_last_bottles(10, conn)?.into_iter().map(|bottle| {
            BottlePage {
                contents: bottle.contents,
                time_pushed: bottle.time_pushed.format(&"%m/%d/%y - %H:%M").to_string(),
                image: bottle.image,
                guild: bottle.guild.map(get_guild_name)
            }
        }).collect()
    };

    Ok(data)
}

fn user(req: &mut Request) -> IronResult<Response> {
    let udata = req.extensions.get::<Router>().unwrap()
        .find("user").and_then(|x| x.parse().ok()).and_then(|uid| {
        get_user_data(uid, &req.get_conn(), &req.get_cfg()).ok()
    });

    match udata {
        Some(udata) => Ok(Response::with((status::Ok, Template::new("user", &udata)))),
        None => Err(IronError::new(ParamError, status::NotFound))
    }
}

fn get_guild_data(gid: GuildId, conn:&Conn, cfg: &Config) -> Res<GuildPage> {
    let gdata = Guild::get(gid, conn);
    let guild: guild::PartialGuild = id::GuildId(gid as u64).to_partial_guild()?;

    let data = GuildPage {
        name: guild.name.clone(), invite: gdata.invite.clone(),
        pfp: guild.icon_url().unwrap_or_else(|| anonymous_url(cfg)).clone(),
        xp: gdata.get_xp(conn)?.unwrap_or(0),
        ranked: gdata.get_ranking(conn).ok(),
        num_bottles: gdata.get_num_bottles(conn)?,
        contributions: gdata.get_contributions(15, conn)?.into_iter().map(|c| {
            GuildContribution {user: get_user_name(c.user), uid: c.user, xp: c.xp}
        }).collect()
    };

    Ok(data)
}

fn guild(req: &mut Request) -> IronResult<Response> {
    let gdata = req.extensions.get::<Router>().unwrap()
        .find("guild").and_then(|x| x.parse().ok()).and_then(|gid| {
        get_guild_data(gid, &req.get_conn(), &req.get_cfg()).ok()
    });

    match gdata {
        Some(gdata) => Ok(Response::with((status::Ok, Template::new("guild", &gdata)))),
        None => Err(IronError::new(ParamError, status::NotFound))
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct DUserData {
    id: String,
    username: String,
    discriminator: String
}

const GETUSER: &str = "https://discordapp.com/api/users/@me";
impl DUserData {
    fn get(access_token: &str) -> Res<Self> {
        use reqwest;

        let res =
            reqwest::Client::new().get(GETUSER).header("Authorization", format!("Bearer {}", access_token)).send()?
            .text()?;

        Ok(serde_json::from_str(&res)?)
    }
}

fn set_session(sesd: SessionData, ses: &mut Session) {
   ses.set(sesd).unwrap()
}

fn get_session(ses: &mut Session) -> SessionData {
    ses.get::<SessionData>().unwrap().unwrap_or_else(|| {
        let sesd = SessionData::new();
        set_session(sesd.clone(), ses);
        sesd
    })
}

fn get_user(ses: &SessionData, conn: &Conn) -> Option<User> {
    User::from_session(ses.id, conn).ok()
}

fn set_tok(ses: &mut Session, tok: oauth2::Token, conn: &Conn) -> Res<()> {
    let uid = DUserData::get(&tok.access_token)?.id.parse()?;
    let mut u = User::get(uid, conn);

    let sesd = get_session(ses);
    u.session = Some(sesd.id);
    u.update(conn)?;

    Ok(())
}

fn report(req: &mut Request) -> IronResult<Response> {
    let bid = req.extensions.get::<Router>().unwrap()
        .find("bottle").and_then(|x| x.parse().ok())
        .ok_or_else(|| IronError::new(ParamError, status::BadRequest))?;

    let conn = &req.get_conn();
    if let Ok (bottle) = Bottle::get(bid, conn) {
        let session = get_session(req.session());
        match get_user(&session, conn) {
            Some(mut x) => {
                let msg = bottle::report_bottle(&bottle, x.id, conn, &req.get_cfg()).unwrap();
                let alreadyexists = Report {user: x.id, bottle: bid, message: msg.id.as_i64()}.make(conn).is_err();

                if !alreadyexists {
                    x.xp += REPORTXP;
                    x.update(conn).unwrap();
                }

                Ok(Response::with((status::Ok, Template::new("reportmade", alreadyexists))))
            },
            None => {
                let mut oauth = req.extensions.get::<DOauth2>().unwrap().clone()
                    .set_state(session.id.to_string());

                set_session(SessionData {redirect: Some(report_url(bid, &req.get_cfg())), ..session}, req.session());
                Ok(Response::with((status::TemporaryRedirect, RedirectRaw (oauth.authorize_url().to_string()))))
            }
        }
    } else {
        Err(IronError::new(ParamError, status::NotFound))
    }
}

fn redirect(req: &mut Request) -> IronResult<Response> {
    let params = req.get_ref::<Params>().unwrap().clone();

    let session = get_session(req.session());
    match params.find(&["state"]) {
        Some(Value::String(state)) if state.clone() == session.id.to_string() => {
            if let Some(Value::String(code)) = params.find(&["code"]) {
                let oauth = req.extensions.get::<DOauth2>().unwrap().clone();

                if let Ok(tok) = oauth.exchange_code(code.clone()) {
                    let conn = &req.get_conn();
                    set_tok(req.session(), tok, conn).unwrap();
                }
            }

            match session.redirect {
                Some(ref redirect) => Ok(Response::with((status::TemporaryRedirect, RedirectRaw(redirect.clone())))),
                _ => Ok(Response::with(status::Ok))
            }
        },

        _ => {
            Err(IronError::new(AuthError, status::BadRequest))
        }
    }
}

fn home(req: &mut Request) -> IronResult<Response> {
    let conn: &Conn = &req.get_conn();

    let data = InternalError::with(|| {
        let mut data = HashMap::new();
        data.insert("bottlecount", get_bottle_count(&conn).map_err(Box::new)?);
        data.insert("usercount", get_user_count(&conn)?);
        data.insert("guildcount", get_guild_count(&conn)?);
        Ok(data)
    })?;

    let resp = Response::with((status::Ok, Template::new("home", &data)));
    Ok(resp)
}

pub fn start_serv (db: ConnPool, cfg: Config) {
    let reqcfg = cfg.clone();
    let oauthcfg = oauth2::Config::new(
        cfg.client_id, cfg.client_secret, "https://discordapp.com/api/oauth2/authorize", "https://discordapp.com/api/oauth2/token"
    )
        .add_scope("identify")
        .set_redirect_url(format!("{}/oauth", cfg.host_url));

    let mut router = Router::new();
    router.get("/", home, "home");
    router.get("/u/:user", user, "user");
    router.get("/g/:guild", guild, "guild");
    router.get("/report/:bottle", report, "report");
    router.get("/oauth", redirect, "redirect");

    let mut chain = Chain::new(router);

    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./res/", ".html")));
    hbse.reload().unwrap();

    chain.link_around(SessionStorage::new(SignedCookieBackend::new(cfg.cookie_sig.into_bytes())));
    chain.link_before(PrerequisiteMiddleware {pool: db, oauth: oauthcfg, cfg: reqcfg});
    chain.link_after(StatusMiddleware);
    chain.link_after(hbse);

    let mut mount = Mount::new();

    mount.mount("/", chain);
    mount.mount("/style", Static::new("./res/style"));
    mount.mount("/img", Static::new("./res/img"));

    let main_mount = if cfg!(debug_assertions) {
        let mut dev_mount = Mount::new();
        dev_mount.mount("/bottle", mount);
        dev_mount
    } else {
        mount
    };

    let iron = Iron::new(main_mount);
    let _ = iron.http("0.0.0.0:8080", ).unwrap();
}