use actix_web::{get, web, HttpRequest, HttpResponse, Responder};

use crate::db::{self, AppData, FullUser, Log, UserMetadata, UserState};

use super::base;
use askama::Template;

#[derive(Default, PartialEq, serde::Deserialize)]
pub struct CallbackQueryProps {
    pub callback: String, // redirect here after finish
}

#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate {
    callback: String,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    site_name: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
struct RegisterTemplate {
    callback: String,
    invite_code_required: bool,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    site_name: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/login_secondary_token.html")]
struct LoginSecondaryTokenTemplate {
    callback: String,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    site_name: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/user_profile.html")]
struct UserProfileTemplate {
    user: UserState<String>,
    meta: UserMetadata,
    user_nick: String,
    can_edit: bool,
    edit_mode: bool,
    about: String,
    is_following: bool,
    followers_count: usize,
    following_count: usize,
    // activity stuff
    activity: Vec<(db::ActivityPost, Vec<db::ActivityPost>, i32)>,
    offset: i32,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    deducktive: String,
    site_name: String,
    body_embed: String,
}

#[derive(Default, PartialEq, serde::Deserialize)]
pub struct QueryProps {
    pub edit: Option<bool>,
    pub offset: Option<i32>,
}

#[derive(Template)]
#[template(path = "auth/activity_post.html")]
struct ViewPostTemplate {
    user: UserState<String>,
    can_edit: bool,
    // post stuff
    post: db::ActivityPost,
    replies: Vec<(db::ActivityPost, Vec<db::ActivityPost>, i32)>,
    favorites_count: i32,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    deducktive: String,
    site_name: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/followers.html")]
struct FollowersTemplate {
    followers: Vec<Log>,
    user: UserState<String>,
    offset: i32,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    site_name: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/following.html")]
struct FollowingTemplate {
    following: Vec<Log>,
    user: UserState<String>,
    offset: i32,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    site_name: String,
    body_embed: String,
}

#[derive(Default, PartialEq, serde::Deserialize)]
pub struct FollowersQueryProps {
    pub offset: Option<i32>,
}

#[derive(Template)]
#[template(path = "auth/user_settings.html")]
struct SettingsTemplate {
    profile: UserState<String>,
    metadata: String,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    site_name: String,
    body_embed: String,
}

#[get("/flow/auth/register")]
/// Available at "/flow/auth/register"
/// Still renders even if `REGISTRATION_DISABLED` is present
pub async fn register_request(
    req: HttpRequest,
    info: web::Query<CallbackQueryProps>,
) -> impl Responder {
    let invite_codes = crate::config::get_var("INVITE_CODES");

    // ...
    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    return HttpResponse::Ok()
        .append_header(("Content-Type", "text/html"))
        .body(
            RegisterTemplate {
                callback: info.callback.clone(),
                invite_code_required: invite_codes.is_some(),
                // required fields
                info: base.info,
                auth_state: base.auth_state,
                bundlrs: base.bundlrs,
                site_name: base.site_name,
                body_embed: base.body_embed,
            }
            .render()
            .unwrap(),
        );
}

#[get("/flow/auth/login")]
/// Available at "/flow/auth/login"
pub async fn login_request(
    req: HttpRequest,
    info: web::Query<CallbackQueryProps>,
) -> impl Responder {
    // ...
    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    return HttpResponse::Ok()
        .append_header(("Content-Type", "text/html"))
        .body(
            LoginTemplate {
                callback: info.callback.clone(),
                // required fields
                info: base.info,
                auth_state: base.auth_state,
                bundlrs: base.bundlrs,
                site_name: base.site_name,
                body_embed: base.body_embed,
            }
            .render()
            .unwrap(),
        );
}

#[get("/flow/auth/login-st")]
/// Available at "/flow/auth/login-st"
pub async fn login_secondary_token_request(
    req: HttpRequest,
    info: web::Query<CallbackQueryProps>,
) -> impl Responder {
    // ...
    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    return HttpResponse::Ok()
        .append_header(("Content-Type", "text/html"))
        .body(
            LoginSecondaryTokenTemplate {
                callback: info.callback.clone(),
                // required fields
                info: base.info,
                auth_state: base.auth_state,
                bundlrs: base.bundlrs,
                site_name: base.site_name,
                body_embed: base.body_embed,
            }
            .render()
            .unwrap(),
        );
}

#[get("/{username:.*}")]
/// Available at "/{username}"
pub async fn profile_view_request(
    req: HttpRequest,
    data: web::Data<AppData>,
    info: web::Query<QueryProps>,
) -> impl Responder {
    // get user
    let username: String = req.match_info().get("username").unwrap().to_string();
    let username_c = username.clone();

    let user: db::DefaultReturn<Option<FullUser<String>>> =
        data.db.get_user_by_username(username).await;

    if user.success == false {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "text/plain"))
            .body("404: Not Found");
    }

    let unwrap = user.payload.as_ref().unwrap();

    // verify auth status
    let (set_cookie, _, token_user) = base::check_auth_status(req.clone(), data.clone()).await;

    // ...
    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());

    // ...
    let followers_res: db::DefaultReturn<usize> =
        data.db.get_user_follow_count(username_c.clone()).await;

    let following_res: db::DefaultReturn<usize> =
        data.db.get_user_following_count(username_c.clone()).await;

    let is_following_res: Option<db::DefaultReturn<Option<db::Log>>> =
        if token_user.is_some() && token_user.as_ref().unwrap().success {
            Option::Some(
                data.db
                    .get_follow_by_user(
                        token_user
                            .as_ref()
                            .unwrap()
                            .payload
                            .as_ref()
                            .unwrap()
                            .user
                            .username
                            .clone(),
                        username_c.clone(),
                    )
                    .await,
            )
        } else {
            Option::None
        };

    let followers_count = followers_res.payload;
    let following_count = following_res.payload;
    let is_following = if is_following_res.is_some() {
        is_following_res.unwrap().payload.is_some()
    } else {
        false
    };

    let user = unwrap.clone().user;
    let edit_mode = if info.edit.is_some() {
        info.edit.unwrap()
    } else {
        false
    };

    // ...
    let meta = serde_json::from_str::<UserMetadata>(&user.metadata).unwrap();
    let active_user = if token_user.is_some() && token_user.as_ref().unwrap().success {
        Option::Some(token_user.unwrap().payload.unwrap().user)
    } else {
        Option::None
    };

    let can_edit = active_user.is_some() && active_user.as_ref().unwrap().username == user.username;

    // activity
    let posts_res: Vec<(db::ActivityPost, Vec<db::ActivityPost>, i32)> = data
        .db
        .get_user_activity(username_c.clone(), info.offset)
        .await
        .payload
        // this really *probably* won't fail
        .unwrap();

    // ...
    let props = UserProfileTemplate {
        user,
        auth_state: base.auth_state,
        info: base.info,
        bundlrs: base.bundlrs,
        deducktive: base.deducktive,
        site_name: base.site_name,
        body_embed: base.body_embed,
        meta: meta.clone(),
        user_nick: if meta.nickname.is_some() {
            meta.nickname.as_ref().unwrap().to_string()
        } else {
            username_c
        },
        can_edit,
        edit_mode,
        about: crate::markup::render(&meta.about.clone()),
        is_following,
        followers_count,
        following_count,
        // activity
        activity: posts_res,
        offset: if info.offset.is_some() {
            info.offset.unwrap()
        } else {
            0
        },
    };

    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(props.render().unwrap());
}

#[get("/{username:.*}/activity/{id}")]
/// Available at "/{username}/activity/{id}"
pub async fn view_post_request(req: HttpRequest, data: web::Data<AppData>) -> impl Responder {
    let post_id: String = req.match_info().get("id").unwrap().to_string();
    let username: String = req.match_info().get("username").unwrap().to_string();

    // get user
    let user: db::DefaultReturn<Option<FullUser<String>>> =
        data.db.get_user_by_username(username).await;

    if user.success == false {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "text/plain"))
            .body("404: Not Found");
    }

    let unwrap = user.payload.as_ref().unwrap();

    // verify auth status
    let (set_cookie, _, token_user) = base::check_auth_status(req.clone(), data.clone()).await;

    // ...
    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    let user = unwrap.clone().user;

    let active_user = if token_user.is_some() && token_user.as_ref().unwrap().success {
        Option::Some(token_user.unwrap().payload.unwrap().user)
    } else {
        Option::None
    };

    let can_edit = active_user.is_some() && active_user.as_ref().unwrap().username == user.username;

    // get post
    let post = data.db.get_post_by_id(post_id.clone()).await;

    if post.success == false {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "text/plain"))
            .body("404: Not Found");
    }

    // activity
    let posts_res = data
        .db
        .get_post_replies_full(post_id.clone(), false)
        .await
        .payload
        // this really *probably* won't fail
        .unwrap();

    // ...
    let props = ViewPostTemplate {
        user,
        can_edit,
        auth_state: base.auth_state,
        info: base.info,
        bundlrs: base.bundlrs,
        deducktive: base.deducktive,
        site_name: base.site_name,
        body_embed: base.body_embed,
        // post
        post: post.payload.unwrap(),
        replies: posts_res,
        favorites_count: data.db.get_post_favorites(post_id).await.payload,
        // TODO: is_favorited
    };

    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(props.render().unwrap());
}

#[get("/{username:.*}/followers")]
/// Available at "/{username}/followers"
pub async fn followers_request(
    req: HttpRequest,
    data: web::Data<AppData>,
    info: web::Query<FollowersQueryProps>,
) -> impl Responder {
    // get user
    let username: String = req.match_info().get("username").unwrap().to_string();
    let username_c = username.clone();

    let user: db::DefaultReturn<Option<FullUser<String>>> =
        data.db.get_user_by_username(username).await;

    if user.success == false {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "text/plain"))
            .body("404: Not Found");
    }

    let unwrap = user.payload.as_ref().unwrap();

    // verify auth status
    let (set_cookie, _, _) = base::check_auth_status(req.clone(), data.clone()).await;

    // ...
    let followers_res: db::DefaultReturn<Option<Vec<db::Log>>> = data
        .db
        .get_user_followers(username_c.clone(), info.offset)
        .await;

    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    let props = FollowersTemplate {
        user: unwrap.clone().user,
        followers: followers_res.payload.unwrap(),
        offset: if info.offset.is_some() {
            info.offset.unwrap()
        } else {
            0
        },
        auth_state: base.auth_state,
        info: base.info,
        bundlrs: base.bundlrs,
        site_name: base.site_name,
        body_embed: base.body_embed,
    };

    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(props.render().unwrap());
}

#[get("/{username:.*}/following")]
/// Available at "/{username}/following"
pub async fn following_request(
    req: HttpRequest,
    data: web::Data<AppData>,
    info: web::Query<FollowersQueryProps>,
) -> impl Responder {
    // get user
    let username: String = req.match_info().get("username").unwrap().to_string();
    let username_c = username.clone();

    let user: db::DefaultReturn<Option<FullUser<String>>> =
        data.db.get_user_by_username(username).await;

    if user.success == false {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "text/plain"))
            .body("404: Not Found");
    }

    let unwrap = user.payload.as_ref().unwrap();

    // verify auth status
    let (set_cookie, _, _) = base::check_auth_status(req.clone(), data.clone()).await;

    // ...
    let following_res: db::DefaultReturn<Option<Vec<db::Log>>> = data
        .db
        .get_user_following(username_c.clone(), info.offset)
        .await;

    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    let props = FollowingTemplate {
        user: unwrap.clone().user,
        following: following_res.payload.unwrap(),
        offset: if info.offset.is_some() {
            info.offset.unwrap()
        } else {
            0
        },
        auth_state: base.auth_state,
        info: base.info,
        bundlrs: base.bundlrs,
        site_name: base.site_name,
        body_embed: base.body_embed,
    };

    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(props.render().unwrap());
}

#[get("/{name:.*}/settings")]
/// Available at "/{name}/settings"
pub async fn user_settings_request(
    req: HttpRequest,
    data: web::Data<db::AppData>,
) -> impl Responder {
    // get user
    let name: String = req.match_info().get("name").unwrap().to_string();
    let profile: db::DefaultReturn<Option<FullUser<String>>> =
        data.db.get_user_by_username(name).await;

    if profile.success == false {
        return HttpResponse::NotFound().body(profile.message);
    }

    let profile = profile.payload.unwrap();

    // verify auth status
    let (set_cookie, token_cookie, token_user) =
        base::check_auth_status(req.clone(), data.clone()).await;

    if token_user.is_none() {
        return HttpResponse::NotAcceptable().body("An account is required to do this");
    }

    // ...
    let user = token_user.unwrap().payload.unwrap();
    let can_view: bool = (user.user.username == profile.user.username)
        | (user
            .level
            .permissions
            .contains(&String::from("ManageUsers")));

    if can_view == false {
        return HttpResponse::NotFound()
            .append_header(("Content-Type", "text/plain"))
            .body("You do not have permission to manage this user's contents.");
    }

    // ...
    let base = base::get_base_values(token_cookie.is_some());
    let props = SettingsTemplate {
        profile: profile.clone().user,
        metadata: profile.user.metadata.replace("/", "\\/"),
        auth_state: base.auth_state,
        info: base.info,
        bundlrs: base.bundlrs,
        site_name: base.site_name,
        body_embed: base.body_embed,
    };

    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(props.render().unwrap());
}
