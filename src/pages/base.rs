use actix_web::{web::Data, HttpRequest};

use crate::db::AppData;
use dorsal::db::special::auth_db::{FullUser, UserMetadata, Result};

pub struct BaseTemplate {
    pub info: String,
    pub auth_state: bool,
    pub bundlrs: String,
    pub deducktive: String,
    pub site_name: String,
    pub body_embed: String,
}

pub fn get_base_values(token_cookie: bool) -> BaseTemplate {
    let info_req = std::env::var("INFO");
    let mut info: String = String::new();

    if info_req.is_err() && info.is_empty() {
        info = "/pub/info".to_string();
    } else {
        info = info_req.unwrap();
    }

    let body_embed_req = std::env::var("BODY_EMBED");
    let body_embed = if body_embed_req.is_ok() {
        body_embed_req.unwrap()
    } else {
        String::new()
    };

    // return
    BaseTemplate {
        info,
        auth_state: token_cookie,
        bundlrs: std::env::var("BUNDLRS_ROOT").unwrap_or(String::new()),
        deducktive: std::env::var("DEDUCKTIVE_ROOT").unwrap_or(String::new()),
        site_name: std::env::var("SITE_NAME").unwrap_or("Guppy".to_string()),
        body_embed,
    }
}

pub async fn check_auth_status(
    req: HttpRequest,
    data: Data<AppData>,
) -> (
    String,
    Option<actix_web::cookie::Cookie<'static>>,
    Option<Result<FullUser<UserMetadata>>>,
) {
    // verify auth status
    let token_cookie = req.cookie("__Secure-Token");
    let mut set_cookie: &str = "";

    let mut token_user: Option<Result<FullUser<UserMetadata>>> = if token_cookie.is_some() {
        Option::Some(
            data.db
                .auth
                .get_user_by_unhashed(token_cookie.as_ref().unwrap().value().to_string()) // if the user is returned, that means the ID is valid
                .await,
        )
    } else {
        Option::None
    };

    if token_user.is_some() {
        // make sure user exists, refresh token if not
        if token_user.as_ref().unwrap().is_ok() == false {
            set_cookie = "__Secure-Token=refresh; SameSite=Strict; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age=0";
            token_user = Option::None;
        }
    }

    // return
    (set_cookie.to_string(), token_cookie, token_user)
}
