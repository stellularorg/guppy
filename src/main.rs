#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://guppy.stellular.net/api/docs/guppy/")]
#![doc(html_favicon_url = "https://stellular.net/static/favicon.svg")]
#![doc(
    html_logo_url = "https://code.stellular.org/repo-avatars/cc8d0efab0759fa6310b75fd5759c33169ee0ab354a958172ed4425a66d2593b"
)]
#![doc(issue_tracker_base_url = "https://code.stellular.org/stellular/guppy/issues/")]

use actix_files as fs;
use actix_web::{web, App, HttpServer};
use dotenv;

pub mod config;
pub mod db;

pub mod api;
pub mod pages;

pub mod markup;

use crate::db::{AppData, Database};
use dorsal::DatabaseOpts;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    // ...
    let args: Vec<String> = config::collect_arguments();

    let port_search: Option<String> = config::get_named_argument(&args, "port");
    let mut port: u16 = 8080;

    if port_search.is_some() {
        port = port_search.unwrap().parse::<u16>().unwrap();
    }

    let static_dir_flag: Option<String> = config::get_named_argument(&args, "static-dir");

    // create database
    let db_type: Option<String> = config::get_named_argument(&args, "db-type");
    let db_host: Option<String> = config::get_var("DB_HOST");
    let db_user: Option<String> = config::get_var("DB_USER");
    let db_pass: Option<String> = config::get_var("DB_PASS");
    let db_name: Option<String> = config::get_var("DB_NAME");

    let db_is_other: bool = db_type
        .clone()
        .is_some_and(|x| (x == String::from("postgres")) | (x == String::from("mysql")));

    if db_is_other && (db_user.is_none() | db_pass.is_none() | db_name.is_none()) {
        panic!("Missing required database config settings!");
    }

    let db: Database = Database::new(DatabaseOpts {
        _type: db_type,
        host: db_host,
        user: if db_is_other {
            db_user.unwrap()
        } else {
            String::new()
        },
        pass: if db_is_other {
            db_pass.unwrap()
        } else {
            String::new()
        },
        name: if db_is_other {
            db_name.unwrap()
        } else {
            String::new()
        },
    })
    .await;

    db.init().await;

    // start server
    println!("Starting server at: http://localhost:{port}");

    // serve routes
    HttpServer::new(move || {
        let client = awc::Client::default();
        let data = web::Data::new(AppData {
            db: db.clone(),
            http_client: client,
        });

        let cors = actix_cors::Cors::default().send_wildcard();

        App::new()
            .app_data(web::Data::clone(&data))
            // middleware
            .wrap(actix_web::middleware::Logger::default())
            .wrap(cors)
            // static dir
            .service(
                fs::Files::new(
                    "/static",
                    if static_dir_flag.is_some() {
                        static_dir_flag.as_ref().unwrap()
                    } else {
                        "./static"
                    },
                )
                .show_files_listing(),
            )
            // docs
            .service(fs::Files::new("/api/docs", "./target/doc").show_files_listing())
            // POST api
            // POST auth
            .service(crate::api::auth::callback_request)
            .service(crate::api::auth::register)
            .service(crate::api::auth::login)
            .service(crate::api::auth::login_secondary_token)
            .service(crate::api::auth::edit_about_request)
            .service(crate::api::auth::refresh_secondary_token_request)
            .service(crate::api::auth::update_request)
            .service(crate::api::auth::follow_request)
            .service(crate::api::auth::ban_request)
            // GET users
            .service(crate::api::auth::avatar_request)
            .service(crate::api::auth::followers_request)
            .service(crate::api::auth::following_request)
            .service(crate::api::auth::level_request)
            // GET dashboard
            .service(crate::pages::auth::register_request)
            .service(crate::pages::auth::login_request)
            .service(crate::pages::auth::login_secondary_token_request)
            // GET root
            .service(crate::api::auth::logout)
            .service(crate::pages::home::home_request)
            // GET users
            .service(crate::pages::auth::followers_request)
            .service(crate::pages::auth::following_request)
            .service(crate::pages::auth::user_settings_request)
            .service(crate::pages::auth::profile_view_request)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
