use actix_web::{delete, get, post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::db::{self, AppData, DefaultReturn, FullUser, UserFollow, UserMetadata};
use dorsal::utility;

#[derive(Default, PartialEq, serde::Deserialize)]
pub struct OffsetQueryProps {
    pub offset: Option<i32>,
}

#[derive(serde::Deserialize)]
struct RegisterInfo {
    username: String,
    invite_code: Option<String>,
}

#[derive(serde::Deserialize)]
struct LoginInfo {
    uid: String,
}

#[derive(serde::Deserialize)]
struct UpdateAboutInfo {
    about: String,
}

#[get("/api/v1/auth/callback")]
/// We also accept the callback on Guppy, but it just redirects here
pub async fn callback_request() -> impl Responder {
    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "text/html"))
        .body(
            "<head>
                <meta http-equiv=\"Refresh\" content=\"0; URL=/\" />
            </head>",
        );
}

#[post("/api/v1/auth/register")]
pub async fn register(body: web::Json<RegisterInfo>, data: web::Data<AppData>) -> impl Responder {
    // if server disabled registration, return
    let disabled = crate::config::get_var("REGISTRATION_DISABLED");

    if disabled.is_some() {
        return HttpResponse::NotAcceptable()
            .body("This server requires has registration disabled.");
    }

    // check invite codes
    let invite_codes = crate::config::get_var("INVITE_CODES");

    if invite_codes.is_some() {
        let invite_codes = invite_codes.unwrap();
        let codes: Vec<&str> = invite_codes.split(",").collect();

        // check body for invite code
        if body.invite_code.is_none() {
            return HttpResponse::NotAcceptable()
                .body("This server requires an invite code to register.");
        }

        let invite_code = body.invite_code.clone().unwrap();

        if codes.contains(&invite_code.as_str()) == false {
            return HttpResponse::NotAcceptable().body("Invalid invite code.");
        }
    }

    // ...
    let username = &body.username.trim();
    let res = data.db.create_user(username.to_string()).await;

    let c = res.clone();
    let set_cookie = if res.success && res.payload.is_some() {
        format!("__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}", c.message, 60 * 60 * 24 * 365)
    } else {
        String::new()
    };

    // return
    return HttpResponse::Ok()
        .append_header(("Set-Cookie", if res.success { &set_cookie } else { "" }))
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string(&res).unwrap());
}

#[post("/api/v1/auth/login")]
pub async fn login(body: web::Json<LoginInfo>, data: web::Data<AppData>) -> impl Responder {
    let id = body.uid.trim();
    let id_hashed = utility::hash(id.to_string());

    let res = data
        .db
        .get_user_by_hashed(id_hashed) // if the user is returned, that means the ID is valid
        .await;

    let set_cookie = if res.is_ok() {
        format!("__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}", body.uid, 60 * 60 * 24 * 365)
    } else {
        String::new()
    };

    if res.is_ok() == false {
        return HttpResponse::NotAcceptable()
            .append_header(("Set-Cookie", if res.is_ok() { &set_cookie } else { "" }))
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<FullUser<UserMetadata>>>(&DefaultReturn {
                    success: true,
                    message: String::new(),
                    payload: res.ok().unwrap(),
                })
                .unwrap(),
            );
    }

    // return
    return HttpResponse::Ok()
        .append_header(("Set-Cookie", if res.is_ok() { &set_cookie } else { "" }))
        .append_header(("Content-Type", "application/json"))
        .body(
            serde_json::to_string(&json! ({
                "success": true,
                "message": body.uid,
            }))
            .unwrap(),
        );
}

#[post("/api/v1/auth/login-st")]
pub async fn login_secondary_token(
    body: web::Json<LoginInfo>,
    data: web::Data<AppData>,
) -> impl Responder {
    let id = body.uid.trim();
    let id_unhashed = id.to_string();

    let res = data
        .db
        .get_user_by_unhashed_st(id_unhashed) // if the user is returned, that means the token is valid
        .await;

    let set_cookie = if res.is_ok() {
        format!("__Secure-Token={}; SameSite=Lax; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age={}", body.uid, 60 * 60 * 24 * 365)
    } else {
        String::new()
    };

    if res.is_ok() == false {
        return HttpResponse::NotAcceptable()
            .append_header(("Set-Cookie", if res.is_ok() { &set_cookie } else { "" }))
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<FullUser<UserMetadata>>>(&DefaultReturn {
                    success: true,
                    message: String::new(),
                    payload: res.ok().unwrap(),
                })
                .unwrap(),
            );
    }

    // return
    return HttpResponse::Ok()
        .append_header(("Set-Cookie", if res.is_ok() { &set_cookie } else { "" }))
        .append_header(("Content-Type", "application/json"))
        .body(
            serde_json::to_string(&json! ({
                "success": true,
                "message": body.uid,
            }))
            .unwrap(),
        );
}

#[get("/api/v1/auth/logout")]
pub async fn logout(req: HttpRequest, data: web::Data<AppData>) -> impl Responder {
    let cookie = req.cookie("__Secure-Token");

    if cookie.is_none() {
        return HttpResponse::NotAcceptable().body("Missing token");
    }

    let res = data
        .db
        .get_user_by_unhashed(cookie.unwrap().value().to_string()) // if the user is returned, that means the ID is valid
        .await;

    if !res.is_ok() {
        return HttpResponse::NotAcceptable().body("Invalid token");
    }

    // return
    return HttpResponse::Ok()
        .append_header(("Set-Cookie", "__Secure-Token=refresh; SameSite=Strict; Secure; Path=/; HostOnly=true; HttpOnly=true; Max-Age=0"))
        .append_header(("Content-Type", "text/plain"))
        .body("You have been signed out. You can now close this tab.");
}

#[get("/api/v1/auth/whoami")]
pub async fn whoami(req: HttpRequest, data: web::Data<AppData>) -> impl Responder {
    let cookie = req.cookie("__Secure-Token");

    if cookie.is_none() {
        // just return nothing on error
        return HttpResponse::Ok().body("");
    }

    let res = data
        .db
        .get_user_by_unhashed(cookie.unwrap().value().to_string()) // if the user is returned, that means the ID is valid
        .await;

    if !res.is_ok() {
        return HttpResponse::Ok().body("");
    }

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "text/plain"))
        .body(res.ok().unwrap().user.username);
}

#[post("/api/v1/auth/users/{name:.*}/about")]
pub async fn edit_about_request(
    req: HttpRequest,
    body: web::Json<UpdateAboutInfo>,
    data: web::Data<AppData>,
) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get token user
    let (_, _, token_user) = crate::pages::base::check_auth_status(req, data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("An account is required to do this");
    }

    let token_user = token_user.unwrap().ok().unwrap();

    // make sure profile exists
    let profile = data.db.get_user_by_username(name.to_owned()).await;

    if !profile.is_ok() {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<Option<String>>>(&DefaultReturn {
                    success: false,
                    message: String::from("Profile does not exist!"),
                    payload: Option::None,
                })
                .unwrap(),
            );
    }

    let profile = profile.ok().unwrap();
    let mut user = profile.user.metadata;

    // check if we can update this user
    // must be authenticated AND same user OR staff
    let can_update: bool = (token_user.user.username == profile.user.username)
        | (token_user
            .level
            .permissions
            .contains(&String::from("ManageUsers")));

    if can_update == false {
        return HttpResponse::NotFound()
            .body("You do not have permission to manage this user's contents.");
    }

    // (check length)
    if (body.about.len() < 2) | (body.about.len() > 200_000) {
        return HttpResponse::Ok()
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<Option<String>>>(&DefaultReturn {
                    success: false,
                    message: String::from("Content is invalid"),
                    payload: Option::None,
                })
                .unwrap(),
            );
    }

    // update about
    user.about = body.about.clone();

    // ...
    let res = data.db.edit_user_metadata_by_name(name, user).await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string(&res).unwrap());
}

#[post("/api/v1/auth/users/{name:.*}/secondary-token")]
pub async fn refresh_secondary_token_request(
    req: HttpRequest,
    data: web::Data<AppData>,
) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get token user
    let (_, _, token_user) = crate::pages::base::check_auth_status(req, data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("An account is required to do this");
    }

    let token_user = token_user.unwrap().ok().unwrap();

    // make sure profile exists
    let profile = data.db.get_user_by_username(name.to_owned()).await;

    if !profile.is_ok() {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<Option<String>>>(&DefaultReturn {
                    success: false,
                    message: String::from("Profile does not exist!"),
                    payload: Option::None,
                })
                .unwrap(),
            );
    }

    let profile = profile.ok().unwrap();
    let mut user = profile.user.metadata;

    // check if we can update this user
    // must be authenticated AND same user OR staff
    let can_update: bool = (token_user.user.username == profile.user.username)
        | (token_user
            .level
            .permissions
            .contains(&String::from("ManageUsers")));

    if can_update == false {
        return HttpResponse::NotFound()
            .body("You do not have permission to manage this user's contents.");
    }

    // update secondary token
    let token = utility::uuid();
    user.secondary_token = Option::Some(utility::hash(token.clone())); // this is essentially just a second ID the user can signin with

    // ...
    let res = data.db.edit_user_metadata_by_name(name, user).await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(
            serde_json::to_string::<DefaultReturn<String>>(&DefaultReturn {
                success: res.success,
                message: res.message,
                payload: token,
            })
            .unwrap(),
        );
}

#[post("/api/v1/auth/users/{name:.*}/follow")]
pub async fn follow_request(req: HttpRequest, data: web::Data<AppData>) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get token user
    let (_, _, token_user) = crate::pages::base::check_auth_status(req, data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("An account is required to do this");
    }

    let token_user = token_user.unwrap().ok().unwrap();

    // ...
    let res = data
        .db
        .toggle_user_follow(&mut UserFollow {
            user: token_user.user.username,
            is_following: name,
        })
        .await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string(&res).unwrap());
}

#[post("/api/v1/auth/users/{name:.*}/update")]
pub async fn update_request(
    req: HttpRequest,
    body: web::Json<UserMetadata>,
    data: web::Data<AppData>,
) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get token user
    let (_, _, token_user) = crate::pages::base::check_auth_status(req, data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("An account is required to do this");
    }

    // make sure profile exists
    let profile = data.db.get_user_by_username(name.to_owned()).await;

    if !profile.is_ok() {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<Option<String>>>(&DefaultReturn {
                    success: false,
                    message: String::from("Profile does not exist!"),
                    payload: Option::None,
                })
                .unwrap(),
            );
    }

    let token_user = token_user.unwrap().ok().unwrap();
    let profile = profile.ok().unwrap();

    // check if we can update this user
    // must be authenticated AND same user OR staff
    let can_update: bool = (token_user.user.username == profile.user.username)
        | (token_user
            .level
            .permissions
            .contains(&String::from("ManageUsers")));

    if can_update == false {
        return HttpResponse::NotFound()
            .body("You do not have permission to manage this user's contents.");
    }

    // ...
    let res = data
        .db
        .edit_user_metadata_by_name(
            name,            // select user
            body.to_owned(), // new metadata
        )
        .await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string(&res).unwrap());
}

#[post("/api/v1/auth/users/{name:.*?}/ban")]
/// Ban user
pub async fn ban_request(req: HttpRequest, data: web::Data<db::AppData>) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get token user
    let (_, _, token_user) = crate::pages::base::check_auth_status(req, data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("An account is required to do this");
    }

    // make sure token_user is of role "staff"
    if !token_user
        .unwrap()
        .ok()
        .unwrap()
        .level
        .permissions
        .contains(&String::from("ManageUsers"))
    {
        return HttpResponse::NotAcceptable().body("Only staff can do this");
    }

    // ban user
    let res: db::DefaultReturn<Option<String>> = data.db.ban_user_by_name(name).await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string::<db::DefaultReturn<Option<String>>>(&res).unwrap());
}

#[get("/api/v1/auth/users/{name:.*}/followers")]
pub async fn followers_request(
    req: HttpRequest,
    data: web::Data<AppData>,
    info: web::Query<OffsetQueryProps>,
) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get followers
    let res: DefaultReturn<Option<Vec<db::Log>>> = data
        .db
        .get_user_followers(name.to_owned(), info.offset)
        .await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string::<DefaultReturn<Option<Vec<db::Log>>>>(&res).unwrap());
}

#[get("/api/v1/auth/users/{name:.*}/following")]
pub async fn following_request(
    req: HttpRequest,
    data: web::Data<AppData>,
    info: web::Query<OffsetQueryProps>,
) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get following
    let res: DefaultReturn<Option<Vec<db::Log>>> = data
        .db
        .get_user_following(name.to_owned(), info.offset)
        .await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string::<DefaultReturn<Option<Vec<db::Log>>>>(&res).unwrap());
}

#[get("/api/v1/auth/users/{name:.*}/avatar")]
pub async fn avatar_request(req: HttpRequest, data: web::Data<AppData>) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // make sure profile exists
    let profile = data.db.get_user_by_username(name.to_owned()).await;

    if !profile.is_ok() {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<DefaultReturn<Option<String>>>(&DefaultReturn {
                    success: false,
                    message: String::from("Profile does not exist!"),
                    payload: Option::None,
                })
                .unwrap(),
            );
    }

    let profile = profile.ok().unwrap();
    let user = profile.user.metadata;

    if user.avatar_url.is_none() {
        return HttpResponse::NotFound().body("User does not have an avatar set");
    }

    let avatar = user.avatar_url.unwrap();

    // fetch avatar
    let res = data
        .http_client
        .get(avatar)
        .timeout(std::time::Duration::from_millis(5_000))
        .insert_header(("User-Agent", "stellular-bundlrs/1.0"))
        .send()
        .await;

    if res.is_err() {
        return HttpResponse::NotFound().body(format!(
            "Failed to fetch avatar on server: {}",
            res.err().unwrap()
        ));
    }

    // ...
    let mut res = res.unwrap();
    let body = res.body().limit(10_000_000).await;

    if body.is_err() {
        return HttpResponse::NotFound().body(
            "Failed to fetch avatar on server (image likely too large, please keep under 10 MB)",
        );
    }

    let body = body.unwrap();

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", res.content_type()))
        .body(body);
}

#[get("/api/v1/auth/users/{name:.*}/level")]
pub async fn level_request(req: HttpRequest, data: web::Data<AppData>) -> impl Responder {
    let name: String = req.match_info().get("name").unwrap().to_string();

    // get user
    let res = data.db.get_user_by_username(name.to_owned()).await;

    if res.is_ok() == false {
        return HttpResponse::Ok()
            .append_header(("Content-Type", "application/json"))
            .body(
                serde_json::to_string::<db::RoleLevel>(&db::RoleLevel {
                    elevation: -1000,
                    name: String::from("anonymous"),
                    permissions: Vec::new(),
                })
                .unwrap(),
            );
    }

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string::<db::RoleLevel>(&res.ok().unwrap().level).unwrap());
}

// activity
#[post("/api/v1/activity")]
pub async fn post_activity_request(
    req: HttpRequest,
    body: web::Json<db::PCreatePost>,
    data: web::Data<AppData>,
) -> impl Responder {
    // get token user
    let (_, _, token_user) = crate::pages::base::check_auth_status(req, data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("An account is required to do this");
    }

    let token_user = token_user.unwrap().ok().unwrap();

    // create props
    let mut props = body.clone();
    props.author = token_user.user.username;

    // post activity
    let res = data.db.create_activity_post(&mut props).await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .body(serde_json::to_string(&res).unwrap());
}

#[post("/api/v1/activity/{id:.*}/favorite")]
/// Toggle a post favorite
pub async fn favorite_request(req: HttpRequest, data: web::Data<db::AppData>) -> impl Responder {
    let post_id = req.match_info().get("id").unwrap();

    // verify auth status
    let (set_cookie, _, token_user) =
        crate::pages::base::check_auth_status(req.clone(), data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable().body("An account is required to favorite posts.");
    }

    // ...
    let res = data
        .db
        .toggle_user_post_favorite(
            token_user.unwrap().ok().unwrap().user.username,
            post_id.to_string(),
        )
        .await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .append_header(("Set-Cookie", set_cookie))
        .body(serde_json::to_string(&res).unwrap());
}

#[delete("/api/v1/activity/{id:.*}")]
/// Delete an activity post
pub async fn delete_activity_request(
    req: HttpRequest,
    data: web::Data<db::AppData>,
) -> impl Responder {
    let post_id = req.match_info().get("id").unwrap();

    // verify auth status
    let (set_cookie, _, token_user) =
        crate::pages::base::check_auth_status(req.clone(), data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable().body("An account is required to manage posts.");
    }

    // ...
    let res = data
        .db
        .delete_activity_post(
            post_id.to_string(),
            Option::Some(token_user.unwrap().ok().unwrap().user.username),
        )
        .await;

    // return
    return HttpResponse::Ok()
        .append_header(("Content-Type", "application/json"))
        .append_header(("Set-Cookie", set_cookie))
        .body(serde_json::to_string(&res).unwrap());
}
