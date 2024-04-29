use actix_web::{get, web, HttpRequest, HttpResponse, Responder};

use crate::db::{self, AppData, FullUser, Log, UserMetadata, UserState};

use super::base;
use askama::Template;
use serde_json::json;

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
    puffer: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
struct RegisterTemplate {
    callback: String,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    puffer: String,
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
    puffer: String,
    body_embed: String,
}

#[derive(Template)]
#[template(path = "auth/user_profile.html")]
struct UserProfileTemplate {
    user: UserState<String>,
    page: String,
    // required fields (super::base)
    info: String,
    auth_state: bool,
    bundlrs: String,
    puffer: String,
    body_embed: String,
}

#[derive(Default, PartialEq, serde::Deserialize)]
pub struct QueryProps {
    pub edit: Option<bool>,
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
    puffer: String,
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
    puffer: String,
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
    puffer: String,
    body_embed: String,
}

#[get("/d/auth/register")]
/// Available at "/d/auth/register"
/// Still renders even if `REGISTRATION_DISABLED` is present
pub async fn register_request(
    req: HttpRequest,
    info: web::Query<CallbackQueryProps>,
) -> impl Responder {
    // ...
    let base = base::get_base_values(req.cookie("__Secure-Token").is_some());
    return HttpResponse::Ok()
        .append_header(("Content-Type", "text/html"))
        .body(
            RegisterTemplate {
                callback: info.callback.clone(),
                // required fields
                info: base.info,
                auth_state: base.auth_state,
                bundlrs: base.bundlrs,
                puffer: base.puffer,
                body_embed: base.body_embed,
            }
            .render()
            .unwrap(),
        );
}

#[get("/d/auth/login")]
/// Available at "/d/auth/login"
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
                puffer: base.puffer,
                body_embed: base.body_embed,
            }
            .render()
            .unwrap(),
        );
}

#[get("/d/auth/login-st")]
/// Available at "/d/auth/login-st"
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
                puffer: base.puffer,
                body_embed: base.body_embed,
            }
            .render()
            .unwrap(),
        );
}

pub fn profile_view_hb_template() -> String {
    String::from("<main class=\"small flex flex-column g-4\">
    <div id=\"error\" class=\"mdnote note-error full\" style=\"display: none\"></div>
    <div id=\"success\" class=\"mdnote note-note full\" style=\"display: none\"></div>

    <div
        class=\"flex justify-space-between align-center mobile:flex-column g-4 flex-wrap\"
    >
        <div class=\"flex align-center g-4 flex-wrap\" style=\"max-width: 100%\">
            {{{ avatar }}} {{{ username_display }}}
        </div>

        {{{ user_actions }}}
    </div>

    <div class=\"card secondary round\">
        <div id=\"stats-or-info\" class=\"flex flex-column g-4\">
            <details class=\"round border\" open>
                <summary>Info</summary>

                <table class=\"full\" style=\"margin: 0\">
                    <thead>
                        <tr>
                            <th>Key</th>
                            <th>Value</th>
                        </tr>
                    </thead>

                    <tbody>
                        <tr>
                            <td>Level</td>
                            <td>{{{ level_badge }}}</td>
                        </tr>
                        <tr>
                            <td>Joined</td>
                            <td>
                                <span class=\"date-time-to-localize\">
                                    {{ user.timestamp }}
                                </span>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </details>

            <details class=\"round border\">
                <summary>Statistics</summary>

                <table class=\"full\" style=\"margin: 0\">
                    <thead>
                        <tr>
                            <th>Key</th>
                            <th>Value</th>
                        </tr>
                    </thead>

                    <tbody>
                        <tr>
                            <td>Followers</td>
                            <td>{{{ followers_button }}}</td>
                        </tr>

                        <tr>
                            <td>Following</td>
                            <td>{{{ following_button }}}</td>
                        </tr>
                    </tbody>
                </table>
            </details>
        </div>

        <hr />

        <div class=\"flex flex-column g-4\">
            <div class=\"card round\" id=\"description\">
                {{{ about }}}
                {{{ edit_about_button }}}
            </div>
        </div>
    </div>
</main>")
}

#[get("/{username:.*}")]
/// Available at "/{username}"
// rustfmt left, we're on our own here
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

    let follower_count = followers_res.payload;
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

    // ...
    let can_edit = active_user.is_some()
        && active_user.as_ref().unwrap().username == user.username;
    
    // template
    let avatar = format!(
        "<img
            class=\"avatar\"
            style=\"--size: {}px;\"
            src=\"/api/auth/users/{}/avatar\"
        />",
        60, user.username
    );
    
    let username_c = user.username.clone();
    let username_display = format!(
        "<div class=\"flex flex-column\" style=\"max-width: 100%; min-width: max-content\">
            <h2 class=\"no-margin\" id=\"user-fake-name\" style=\"max-width: 100vw\">{}</h2>
        
            <span id=\"user-real-name\">{}</span>
        </div>",
        if meta.nickname.is_some() {
            meta.nickname.as_ref().unwrap()
        } else {
            &username_c
        },
        user.username
    );
    
    let about = if edit_mode == true {
        // edit mode form
        format!("<form id=\"edit-about\" class=\"flex flex-column g-4\" data-endpoint=\"/api/auth/users/{}/about\">
            <div class=\"full flex justify-space-between align-center g-4\">
                <b>Edit About</b>
        
                <button class=\"theme:primary round\">
                    <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"lucide lucide-save\"><path d=\"M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z\"/><polyline points=\"17 21 17 13 7 13 7 21\"/><polyline points=\"7 3 7 8 15 8\"/></svg>
                    Save
                </button>
            </div>
        
            <textarea
                type=\"text\"
                name=\"about\"
                id=\"about\"
                placeholder=\"About\"
                class=\"full round\"
                minlength=\"2\"
                maxlength=\"200000\"
                required
            >{}</textarea>
        </form>", user.username, meta.about)
    } else {
        // just show about
        crate::markup::render(&meta.about.clone())
    };
    
    let edit_about_button = if can_edit == true && edit_mode == false {
        "<a class=\"button round theme:primary\" href=\"?edit=true\">Edit About</a>"
    } else {
        ""
    };
    
    let followers_button = format!(
        "<a href=\"/{}/followers\">{}</a>",
        user.username, follower_count
    );
    
    let following_button = format!(
        "<a href=\"/{}/following\">{}</a>",
        user.username, following_count
    );
    
    let user_actions = format!(
        "{}",
        if (can_edit == false)
            && (base.auth_state == true)
        {
            format!(
                "<div class=\"flex flex-wrap g-4\">
                    <button class=\"round theme:primary\" id=\"mail-user\" data-endpoint=\"/api/auth/users/{}/mail\">Mail</button>
                    <button class=\"round theme:primary\" id=\"follow-user\" data-endpoint=\"/api/auth/users/{}/follow\">{}</button>
                </div>", 
                user.username, 
                user.username, 
                if is_following == false {
                    "Follow"
                } else {
                    "Unfollow"
                }
            )
        } else {
            String::new()
        }
    );
    
    let level_badge = format!("<span class=\"chip badge role-{}\">{}</span>", user.role, user.role);
    
    // render template
    let default_template = &profile_view_hb_template();
    let reg = handlebars::Handlebars::new();
    let page = reg.render_template(
        if meta.page_template.is_some() && !meta.page_template.as_ref().unwrap().is_empty() {
            meta.page_template.as_ref().unwrap() // use provided template
        } else {
            default_template // use default template
        },
        &json!({
            // user info
            "username": user.username,
            "about": about,
            // components
            "avatar": avatar,
            "username_display": username_display,
            "edit_about_button": edit_about_button,
            "followers_button": followers_button,
            "following_button": following_button,
            "user_actions": user_actions,
            "level_badge": level_badge,
            // ...
            "user": user,
            "metadata": meta.clone()
        }),
    );

    if page.is_err() {
        return HttpResponse::NotAcceptable()
            .append_header(("Content-Type", "text/plain"))
            .body("Failed to render template");
    }

    // ...
    // TODO: properly sanitize if needed
    // rustfmt i miss you
    let page =
        page.unwrap().replace("fetch(", "fetch(\\");

    // ...
    let props = UserProfileTemplate {
        user,
        page,
        auth_state: base.auth_state,
        info: base.info,
        bundlrs: base.bundlrs,
        puffer: base.puffer,
        body_embed: base.body_embed,
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
        puffer: base.puffer,
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
        puffer: base.puffer,
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
    let (set_cookie, token_cookie, token_user) = base::check_auth_status(req.clone(), data.clone()).await;

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
        puffer: base.puffer,
        body_embed: base.body_embed,
    };

    return HttpResponse::Ok()
        .append_header(("Set-Cookie", set_cookie))
        .append_header(("Content-Type", "text/html"))
        .body(props.render().unwrap());
}