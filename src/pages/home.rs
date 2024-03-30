use actix_web::{get, web, HttpRequest, HttpResponse, Responder};

use crate::db::guppydb;

use super::base;
use askama::Template;

#[derive(Template)]
#[template(path = "homepage.html")]
struct HomeTemplate {
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    puffer: String,
}

#[get("/")]
pub async fn home_request(req: HttpRequest, data: web::Data<guppydb::AppData>) -> impl Responder {
    // verify auth status
    let token_cookie = req.cookie("__Secure-Token");
    let mut set_cookie: &str = "";

    let mut token_user = if token_cookie.is_some() {
        Option::Some(
            data.db
                .get_user_by_unhashed(token_cookie.as_ref().unwrap().value().to_string()) // if the user is returned, that means the ID is valid
                .await,
        )
    } else {
        Option::None
    };

    if token_user.is_some() {
        // make sure user exists, refresh token if not
        if token_user.as_ref().unwrap().success == false {
            set_cookie = "__Secure-Token=refresh; SameSite=Strict; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age=0";
            token_user = Option::None;
        }
    }

    // ...
    let base = base::get_base_values(token_user.is_some());
    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(
            HomeTemplate {
                // required fields
                info: base.info,
                auth_state: base.auth_state,
                bundlrs: base.bundlrs,
                puffer: base.puffer,
            }
            .render()
            .unwrap(),
        );
}
