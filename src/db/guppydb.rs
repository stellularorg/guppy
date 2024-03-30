//! # GuppyDB
//! Database handler for all database types
use super::{
    cachedb::CacheDB,
    sql::{self, Database, DatabaseOpts},
};

use sqlx::{Column, Row};

use crate::utility;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Clone)]
pub struct AppData {
    pub db: GuppyDB,
    pub http_client: awc::Client,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
/// Default API return value
pub struct DefaultReturn<T> {
    pub success: bool,
    pub message: String,
    pub payload: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseReturn {
    pub data: HashMap<String, String>,
}

#[derive(Debug, Default, PartialEq, sqlx::FromRow, Clone, Serialize, Deserialize)]
/// A user object
pub struct UserState<M> {
    // selectors
    pub username: String,
    pub id_hashed: String, // users use their UNHASHED id to login, it is used as their session id too!
    //                        the hashed id is the only id that should ever be public!
    pub role: String,
    // dates
    pub timestamp: u128,
    // ...
    pub metadata: M,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoleLevel {
    pub elevation: i32, // this marks the level of the role, 0 should always be member
    // users cannot manage users of a higher elevation than them
    pub name: String,             // role name, shown on user profiles
    pub permissions: Vec<String>, // a vec of user permissions (ex: "ManagePastes")
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct FullUser<M> {
    pub user: UserState<M>,
    pub level: RoleLevel,
}

#[derive(Default, Clone, sqlx::FromRow, Serialize, Deserialize, PartialEq)]
pub struct RoleLevelLog {
    pub id: String,
    pub level: RoleLevel,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMetadata {
    pub about: String,
    pub avatar_url: Option<String>,
    pub secondary_token: Option<String>,
    pub allow_mail: Option<String>,    // yes/no
    pub nickname: Option<String>,      // user display name
    pub page_template: Option<String>, // profile handlebars template
}

#[derive(Default, PartialEq, sqlx::FromRow, Clone, Serialize, Deserialize)]
pub struct Log {
    // selectors
    pub id: String,
    pub logtype: String,
    // dates
    pub timestamp: u128,
    // ...
    pub content: String,
}

#[derive(Debug, Default, sqlx::FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogIdentifier {
    pub id: String,
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserFollow {
    pub user: String,         // the user that is following `is_following`
    pub is_following: String, // use user that `user` is following
}

#[allow(dead_code)]
pub fn deserialize_userfollow(input: String) -> UserFollow {
    serde_json::from_str::<UserFollow>(&input).unwrap()
}

// ...
#[derive(Clone)]
#[cfg(feature = "postgres")]
pub struct GuppyDB {
    pub db: Database<sqlx::PgPool>,
    pub options: DatabaseOpts,
    pub cachedb: CacheDB,
}

#[derive(Clone)]
#[cfg(feature = "mysql")]
pub struct GuppyDB {
    pub db: Database<sqlx::MySqlPool>,
    pub options: DatabaseOpts,
    pub cachedb: CacheDB,
}

#[derive(Clone)]
#[cfg(feature = "sqlite")]
pub struct GuppyDB {
    pub db: Database<sqlx::SqlitePool>,
    pub options: DatabaseOpts,
    pub cachedb: CacheDB,
}

impl GuppyDB {
    pub async fn new(options: DatabaseOpts) -> GuppyDB {
        return GuppyDB {
            db: sql::create_db(options.clone()).await,
            options,
            cachedb: CacheDB::new().await,
        };
    }

    pub async fn init(&self) {
        // create tables
        let c = &self.db.client;

        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS \"Users\" (
                username VARCHAR(1000000),
                id_hashed VARCHAR(1000000),
                role VARCHAR(1000000),
                timestamp VARCHAR(1000000),
                metadata VARCHAR(1000000)
            )",
        )
        .execute(c)
        .await;

        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS \"Logs\" (
                id VARCHAR(1000000),
                logtype VARCHAR(1000000),
                timestamp  VARCHAR(1000000),
                content VARCHAR(1000000)
            )",
        )
        .execute(c)
        .await;
    }

    #[cfg(feature = "sqlite")]
    fn textify_row(&self, row: sqlx::sqlite::SqliteRow) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: HashMap<String, String> = HashMap::new();

        for column in columns {
            let value = row.get(column.name());
            out.insert(column.name().to_string(), value);
        }

        // return
        return DatabaseReturn { data: out };
    }

    #[cfg(feature = "postgres")]
    fn textify_row(&self, row: sqlx::postgres::PgRow) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: HashMap<String, String> = HashMap::new();

        for column in columns {
            let value = row.get(column.name());
            out.insert(column.name().to_string(), value);
        }

        // return
        return DatabaseReturn { data: out };
    }

    #[cfg(feature = "mysql")]
    fn textify_row(&self, row: sqlx::mysql::MySqlRow) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: HashMap<String, String> = HashMap::new();

        for column in columns {
            let value = row.try_get::<Vec<u8>, _>(column.name());

            if value.is_ok() {
                // returned bytes instead of text :(
                // we're going to convert this to a string and then add it to the output!
                out.insert(
                    column.name().to_string(),
                    std::str::from_utf8(value.unwrap().as_slice())
                        .unwrap()
                        .to_string(),
                );
            } else {
                // already text
                let value = row.get(column.name());
                out.insert(column.name().to_string(), value);
            }
        }

        // return
        return DatabaseReturn { data: out };
    }

    // users

    // GET
    /// Get a user by their hashed ID
    ///
    /// # Arguments:
    /// * `hashed` - `String` of the user's hashed ID
    pub async fn get_user_by_hashed(
        &self,
        hashed: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        // fetch from database
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Users\" WHERE \"id_hashed\" = ?"
        } else {
            "SELECT * FROM \"Users\" WHERE \"id_hashed\" = $1"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&hashed)
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.textify_row(row).data;

        let role = row.get("role").unwrap().to_string();
        if role == "banned" {
            return DefaultReturn {
                success: false,
                message: String::from("User is banned"),
                payload: Option::None,
            };
        }

        // ...
        let meta = row.get("metadata"); // for compatability - users did not have metadata until Bundlrs v0.10.6
        let user = UserState {
            username: row.get("username").unwrap().to_string(),
            id_hashed: row.get("id_hashed").unwrap().to_string(),
            role: role.clone(),
            timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            metadata: if meta.is_some() {
                meta.unwrap().to_string()
            } else {
                String::new()
            },
        };

        // fetch level from role
        let level = self.get_level_by_role(role).await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("User exists"),
            payload: Option::Some(FullUser {
                user,
                level: level.payload.level,
            }),
        };
    }

    /// Get a user by their unhashed ID (hashes ID and then calls [`GuppyDB::get_user_by_hashed()`])
    ///
    /// Calls [`GuppyDB::get_user_by_unhashed_st()`] if user is invalid.
    ///
    /// # Arguments:
    /// * `unhashed` - `String` of the user's unhashed ID
    pub async fn get_user_by_unhashed(
        &self,
        unhashed: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        let res = self
            .get_user_by_hashed(utility::hash(unhashed.clone()))
            .await;

        if res.success == false {
            // treat unhashed as a secondary token and try again
            return self.get_user_by_unhashed_st(unhashed).await;
        }

        res
    }

    /// Get a user by their unhashed secondary token
    ///
    /// # Arguments:
    /// * `unhashed` - `String` of the user's unhashed secondary token
    pub async fn get_user_by_unhashed_st(
        &self,
        unhashed: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        // fetch from database
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Users\" WHERE \"metadata\" LIKE ?"
        } else {
            "SELECT * FROM \"Users\" WHERE \"metadata\" LIKE $1"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!(
                "%\"secondary_token\":\"{}\"%",
                crate::utility::hash(unhashed)
            ))
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.textify_row(row).data;

        let role = row.get("role").unwrap().to_string();
        if role == "banned" {
            return DefaultReturn {
                success: false,
                message: String::from("User is banned"),
                payload: Option::None,
            };
        }

        // ...
        let meta = row.get("metadata"); // for compatability - users did not have metadata until Bundlrs v0.10.6
        let user = UserState {
            username: row.get("username").unwrap().to_string(),
            id_hashed: row.get("id_hashed").unwrap().to_string(),
            role: role.clone(),
            timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            metadata: if meta.is_some() {
                meta.unwrap().to_string()
            } else {
                String::new()
            },
        };

        // fetch level from role
        let level = self.get_level_by_role(role).await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("User exists"),
            payload: Option::Some(FullUser {
                user,
                level: level.payload.level,
            }),
        };
    }

    /// Get a user by their username
    ///
    /// # Arguments:
    /// * `username` - `String` of the user's username
    pub async fn get_user_by_username(
        &self,
        username: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        // check in cache
        let cached = self.cachedb.get(format!("user:{}", username)).await;

        if cached.is_some() {
            // ...
            let user = serde_json::from_str::<UserState<String>>(cached.unwrap().as_str()).unwrap();

            // get role
            let role = user.role.clone();
            if role == "banned" {
                // account banned - we're going to act like it simply does not exist
                return DefaultReturn {
                    success: false,
                    message: String::from("User is banned"),
                    payload: Option::None,
                };
            }

            // fetch level from role
            let level = self.get_level_by_role(role.clone()).await;

            // ...
            return DefaultReturn {
                success: true,
                message: String::from("User exists (cache)"),
                payload: Option::Some(FullUser {
                    user,
                    level: level.payload.level,
                }),
            };
        }

        // ...
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Users\" WHERE \"username\" = ?"
        } else {
            "SELECT * FROM \"Users\" WHERE \"username\" = $1"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&username)
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.textify_row(row).data;

        let role = row.get("role").unwrap().to_string();
        if role == "banned" {
            // account banned - we're going to act like it simply does not exist
            return DefaultReturn {
                success: false,
                message: String::from("User is banned"),
                payload: Option::None,
            };
        }

        // fetch level from role
        let level = self.get_level_by_role(role.clone()).await;

        // store in cache
        let meta = row.get("metadata");
        let user = UserState {
            username: row.get("username").unwrap().to_string(),
            id_hashed: row.get("id_hashed").unwrap().to_string(),
            role,
            timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            metadata: if meta.is_some() {
                meta.unwrap().to_string()
            } else {
                String::new()
            },
        };

        self.cachedb
            .set(
                format!("user:{}", username),
                serde_json::to_string::<UserState<String>>(&user).unwrap(),
            )
            .await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("User exists (new)"),
            payload: Option::Some(FullUser {
                user,
                level: level.payload.level,
            }),
        };
    }

    /// Get a [`RoleLevel`] by its `name`
    ///
    /// # Arguments:
    /// * `name` - `String` of the level's role name
    pub async fn get_level_by_role(&self, name: String) -> DefaultReturn<RoleLevelLog> {
        // check if level already exists in cache
        let cached = self.cachedb.get(format!("level:{}", name)).await;

        if cached.is_some() {
            return DefaultReturn {
                success: true,
                message: String::from("Level exists (cache)"),
                payload: serde_json::from_str::<RoleLevelLog>(cached.unwrap().as_str()).unwrap(),
            };
        }

        // ...
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"logtype\" = 'level' AND \"content\" LIKE ?"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"logtype\" = 'level' AND \"content\" LIKE $1"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!("%\"name\":\"{}\"%", name))
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: true,
                message: String::from("Level does not exist, using default"),
                payload: RoleLevelLog {
                    id: String::new(),
                    level: RoleLevel {
                        name: String::from("member"),
                        elevation: 0,
                        permissions: Vec::new(),
                    },
                },
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.textify_row(row).data;

        // store in cache
        let id = row.get("id").unwrap().to_string();
        let level = serde_json::from_str::<RoleLevel>(row.get("content").unwrap()).unwrap();

        let level = RoleLevelLog { id, level };
        self.cachedb
            .set(
                format!("level:{}", name),
                serde_json::to_string::<RoleLevelLog>(&level).unwrap(),
            )
            .await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Level exists (new)"),
            payload: level,
        };
    }

    // SET
    /// Create a new user given their username. Returns their hashed ID
    ///
    /// # Arguments:
    /// * `username` - `String` of the user's `username`
    pub async fn create_user(&self, username: String) -> DefaultReturn<Option<String>> {
        // make sure user doesn't already exists
        let existing = &self.get_user_by_username(username.clone()).await;
        if existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("User already exists!"),
                payload: Option::None,
            };
        }

        // check username
        let regex = regex::RegexBuilder::new("^[\\w\\_\\-\\.\\!]+$")
            .multi_line(true)
            .build()
            .unwrap();

        if regex.captures(&username).iter().len() < 1 {
            return DefaultReturn {
                success: false,
                message: String::from("Username is invalid"),
                payload: Option::None,
            };
        }

        if (username.len() < 2) | (username.len() > 500) {
            return DefaultReturn {
                success: false,
                message: String::from("Username is invalid"),
                payload: Option::None,
            };
        }

        // ...
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "INSERT INTO \"Users\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"Users\" VALUES ($1, $2, $3, $4, $5)"
        };

        let user_id_unhashed: String = utility::uuid();
        let user_id_hashed: String = utility::hash(user_id_unhashed.clone());
        let timestamp = utility::unix_epoch_timestamp().to_string();

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&username)
            .bind::<&String>(&user_id_hashed)
            .bind::<&String>(&String::from("member")) // default role
            .bind::<&String>(&timestamp)
            .bind::<&String>(
                &serde_json::to_string::<UserMetadata>(&UserMetadata {
                    about: String::new(),
                    avatar_url: Option::None,
                    secondary_token: Option::None,
                    allow_mail: Option::Some(String::from("yes")),
                    nickname: Option::Some(username.clone()),
                    page_template: Option::None,
                })
                .unwrap(),
            )
            .execute(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // return
        return DefaultReturn {
            success: true,
            message: user_id_unhashed,
            payload: Option::Some(user_id_hashed),
        };
    }

    /// Update a [`UserState`]'s metadata by its `username`
    pub async fn edit_user_metadata_by_name(
        &self,
        name: String,
        metadata: UserMetadata,
    ) -> DefaultReturn<Option<String>> {
        // make sure user exists
        let existing = &self.get_user_by_username(name.clone()).await;
        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist!"),
                payload: Option::None,
            };
        }

        // update user
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "UPDATE \"Users\" SET \"metadata\" = ? WHERE \"username\" = ?"
        } else {
            "UPDATE \"Users\" SET (\"metadata\") = ($1) WHERE \"username\" = $2"
        };

        let c = &self.db.client;
        let meta = &serde_json::to_string(&metadata).unwrap();
        let res = sqlx::query(query)
            .bind::<&String>(meta)
            .bind::<&String>(&name)
            .execute(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // update cache
        let existing_in_cache = self.cachedb.get(format!("user:{}", name)).await;

        if existing_in_cache.is_some() {
            let mut user =
                serde_json::from_str::<UserState<String>>(&existing_in_cache.unwrap()).unwrap();
            user.metadata = meta.to_string(); // update metadata

            // update cache
            self.cachedb
                .update(
                    format!("user:{}", name),
                    serde_json::to_string::<UserState<String>>(&user).unwrap(),
                )
                .await;
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("User updated!"),
            payload: Option::Some(name),
        };
    }

    /// Ban a [`UserState`] by its `username`
    pub async fn ban_user_by_name(&self, name: String) -> DefaultReturn<Option<String>> {
        // make sure user exists
        let existing = &self.get_user_by_username(name.clone()).await;
        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist!"),
                payload: Option::None,
            };
        }

        // make sure user level elevation is 0
        let level = &existing.payload.as_ref().unwrap().level;
        if level.elevation == 0 {
            return DefaultReturn {
                success: false,
                message: String::from("User must be of level elevation 0"),
                payload: Option::None,
            };
        }

        // update user
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "UPDATE \"Users\" SET \"role\" = ? WHERE \"username\" = ?"
        } else {
            "UPDATE \"Users\" SET (\"role\") = ($1) WHERE \"username\" = $2"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&str>("banned")
            .bind::<&String>(&name)
            .execute(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // update cache
        let existing_in_cache = self.cachedb.get(format!("user:{}", name)).await;

        if existing_in_cache.is_some() {
            let mut user =
                serde_json::from_str::<UserState<String>>(&existing_in_cache.unwrap()).unwrap();
            user.role = String::from("banned"); // update role

            // update cache
            self.cachedb
                .update(
                    format!("user:{}", name),
                    serde_json::to_string::<UserState<String>>(&user).unwrap(),
                )
                .await;
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("User banned!"),
            payload: Option::Some(name),
        };
    }

    /*
    /// Create a new [`RoleLevel`] given various properties
    ///
    /// # Arguments:
    /// * `props` - [`RoleLevel`]
    pub async fn create_level(&self, props: &mut RoleLevel) -> DefaultReturn<RoleLevelLog> {
        let p: &mut RoleLevel = props; // borrowed props

        // make sure role does not exist
        let existing: DefaultReturn<RoleLevelLog> = self.get_level_by_role(p.name.to_owned()).await;

        if existing.success {
            return existing;
        }

        // create role
        let res = self
            .create_log(
                String::from("level"),
                serde_json::to_string::<RoleLevel>(p).unwrap(),
            )
            .await;

        // return
        return DefaultReturn {
            success: res.success,
            message: res.message,
            payload: RoleLevelLog {
                id: if res.success {
                    res.payload.unwrap()
                } else {
                    String::new()
                },
                level: p.to_owned(),
            },
        };
    }

    /// Delete an existing [`RoleLevel`] given its `name`
    ///
    /// # Arguments:
    /// * `name` - [`RoleLevel`] `name` field
    pub async fn delete_level(&self, props: &mut RoleLevel) -> DefaultReturn<Option<String>> {
        let p: &mut RoleLevel = props; // borrowed props

        // make sure role exists
        let existing: DefaultReturn<RoleLevelLog> = self.get_level_by_role(p.name.to_owned()).await;

        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("Level does not exist"),
                payload: Option::None,
            };
        }

        // delete role
        let res = self.delete_log(existing.payload.id.clone()).await;

        // return
        return DefaultReturn {
            success: res.success,
            message: res.message,
            payload: Option::Some(existing.payload.id),
        };
    }
    */

    // logs

    // GET
    /// Get a log by its id
    ///
    /// # Arguments:
    /// * `id` - `String` of the log's `id`
    pub async fn get_log_by_id(&self, id: String) -> DefaultReturn<Option<Log>> {
        // check in cache
        let cached = self.cachedb.get(format!("log:{}", id)).await;

        if cached.is_some() {
            // ...
            let log = serde_json::from_str::<Log>(cached.unwrap().as_str()).unwrap();

            // return
            return DefaultReturn {
                success: true,
                message: String::from("Log exists (cache)"),
                payload: Option::Some(log),
            };
        }

        // ...
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"id\" = $1"
        };

        let c = &self.db.client;
        let res = sqlx::query(query).bind::<&String>(&id).fetch_one(c).await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Log does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.textify_row(row).data;

        // store in cache
        let log = Log {
            id: row.get("id").unwrap().to_string(),
            logtype: row.get("logtype").unwrap().to_string(),
            timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            content: row.get("content").unwrap().to_string(),
        };

        self.cachedb
            .set(
                format!("log:{}", id),
                serde_json::to_string::<Log>(&log).unwrap(),
            )
            .await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Paste exists"),
            payload: Option::Some(log),
        };
    }

    // SET
    /// Create a log given its type and content
    ///
    /// # Arguments:
    /// * `logtype` - `String` of the log's `logtype`
    /// * `content` - `String` of the log's `content`
    pub async fn create_log(
        &self,
        logtype: String,
        content: String,
    ) -> DefaultReturn<Option<String>> {
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "INSERT INTO \"Logs\" VALUES (?, ?, ?, ?)"
        } else {
            "INSERT INTO \"Logs\" VALUES ($1, $2, $3, $4)"
        };

        let log_id: String = utility::random_id();

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&log_id)
            .bind::<String>(logtype)
            .bind::<String>(utility::unix_epoch_timestamp().to_string())
            .bind::<String>(content)
            .execute(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Log created!"),
            payload: Option::Some(log_id),
        };
    }

    /// Edit a log given its ID
    ///
    /// # Arguments:
    /// * `id` - `String` of the log's `id`
    /// * `content` - `String` of the log's new content
    pub async fn edit_log(&self, id: String, content: String) -> DefaultReturn<Option<String>> {
        // make sure log exists
        let existing = &self.get_log_by_id(id.clone()).await;
        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("Log does not exist!"),
                payload: Option::None,
            };
        }

        // update log
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "UPDATE \"Logs\" SET \"content\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"Logs\" SET (\"content\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&content)
            .bind::<&String>(&id)
            .execute(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // update cache
        let existing_in_cache = self.cachedb.get(format!("log:{}", id)).await;

        if existing_in_cache.is_some() {
            let mut log = serde_json::from_str::<Log>(&existing_in_cache.unwrap()).unwrap();

            log.content = content; // update content

            // update cache
            self.cachedb
                .update(
                    format!("log:{}", id),
                    serde_json::to_string::<Log>(&log).unwrap(),
                )
                .await;
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Log updated!"),
            payload: Option::Some(id),
        };
    }

    /// Delete a log given its id
    ///
    /// # Arguments:
    /// * `id` - `String` of the log's `id`
    pub async fn delete_log(&self, id: String) -> DefaultReturn<Option<String>> {
        // make sure log exists
        let existing = &self.get_log_by_id(id.clone()).await;
        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("Log does not exist!"),
                payload: Option::None,
            };
        }

        // update log
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "DELETE FROM \"Logs\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"Logs\" WHERE \"id\" = $1"
        };

        let c = &self.db.client;
        let res = sqlx::query(query).bind::<&String>(&id).execute(c).await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // update cache
        self.cachedb.remove(format!("log:{}", id)).await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Log deleted!"),
            payload: Option::Some(id),
        };
    }

    // follows

    // GET
    /// Get a [`UserFollow`] by the username of the user following
    ///
    /// # Arguments:
    /// * `user` - username of user following
    /// * `is_following` - the username of the user that `user` is following
    pub async fn get_follow_by_user(
        &self,
        user: String,
        is_following: String,
    ) -> DefaultReturn<Option<Log>> {
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow'"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!(
                "%\"user\":\"{user}\",\"is_following\":\"{is_following}\"%"
            ))
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Follow does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.textify_row(row).data;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Follow exists"),
            payload: Option::Some(Log {
                id: row.get("id").unwrap().to_string(),
                logtype: row.get("logtype").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
                content: row.get("content").unwrap().to_string(),
            }),
        };
    }

    /// Get the [`UserFollow`]s that are following the given `user`
    ///
    /// # Arguments:
    /// * `user` - username of user to check
    /// * `offset` - optional value representing the SQL fetch offset
    pub async fn get_user_followers(
        &self,
        user: String,
        offset: Option<i32>,
    ) -> DefaultReturn<Option<Vec<Log>>> {
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET ?"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET $2"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!("%\"is_following\":\"{user}\"%"))
            .bind(if offset.is_some() { offset.unwrap() } else { 0 })
            .fetch_all(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch followers"),
                payload: Option::None,
            };
        }

        // ...
        let rows = res.unwrap();
        let mut output: Vec<Log> = Vec::new();

        for row in rows {
            let row = self.textify_row(row).data;
            output.push(Log {
                id: row.get("id").unwrap().to_string(),
                logtype: row.get("logtype").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
                content: row.get("content").unwrap().to_string(),
            });
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Followers exists"),
            payload: Option::Some(output),
        };
    }

    /// Get the [`UserFollow`]s that the given `user` is following
    ///
    /// # Arguments:
    /// * `user` - username of user to check
    /// * `offset` - optional value representing the SQL fetch offset
    pub async fn get_user_following(
        &self,
        user: String,
        offset: Option<i32>,
    ) -> DefaultReturn<Option<Vec<Log>>> {
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET ?"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET $2"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!("%\"user\":\"{user}\"%"))
            .bind(if offset.is_some() { offset.unwrap() } else { 0 })
            .fetch_all(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch following"),
                payload: Option::None,
            };
        }

        // ...
        let rows = res.unwrap();
        let mut output: Vec<Log> = Vec::new();

        for row in rows {
            let row = self.textify_row(row).data;
            output.push(Log {
                id: row.get("id").unwrap().to_string(),
                logtype: row.get("logtype").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
                content: row.get("content").unwrap().to_string(),
            });
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Following exists"),
            payload: Option::Some(output),
        };
    }

    /// Get the amount of followers a user has
    ///
    /// # Arguments:
    /// * `user` - username of user to check
    pub async fn get_user_follow_count(&self, user: String) -> DefaultReturn<usize> {
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow'"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!("%\"is_following\":\"{user}\"%"))
            .fetch_all(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch followers"),
                payload: 0,
            };
        }

        // ...
        let rows = res.unwrap();

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Follow exists"),
            payload: rows.len(),
        };
    }

    /// Get the amount of users a user is following
    ///
    /// # Arguments:
    /// * `user` - username of user to check
    pub async fn get_user_following_count(&self, user: String) -> DefaultReturn<usize> {
        let query: &str = if (self.db._type == "sqlite") | (self.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow'"
        };

        let c = &self.db.client;
        let res = sqlx::query(query)
            .bind::<&String>(&format!("%\"user\":\"{user}\"%"))
            .fetch_all(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch following"),
                payload: 0,
            };
        }

        // ...
        let rows = res.unwrap();

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Follow exists"),
            payload: rows.len(),
        };
    }

    // SET
    /// Toggle the following status of `user` on `is_following` ([`UserFollow`])
    ///
    /// # Arguments:
    /// * `props` - [`UserFollow`]
    pub async fn toggle_user_follow(
        &self,
        props: &mut UserFollow,
    ) -> DefaultReturn<Option<String>> {
        let p: &mut UserFollow = props; // borrowed props

        // users cannot be the same
        if p.user == p.is_following {
            return DefaultReturn {
                success: false,
                message: String::from("Cannot follow yourself!"),
                payload: Option::None,
            };
        }

        // make sure both users exist
        let existing: DefaultReturn<Option<FullUser<String>>> =
            self.get_user_by_username(p.user.to_owned()).await;

        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist!"),
                payload: Option::None,
            };
        }

        // make sure both users exist
        let existing: DefaultReturn<Option<FullUser<String>>> =
            self.get_user_by_username(p.is_following.to_owned()).await;

        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("User (2) does not exist!"),
                payload: Option::None,
            };
        }

        // check if follow exists
        let existing: DefaultReturn<Option<Log>> = self
            .get_follow_by_user(p.user.to_owned(), p.is_following.to_owned())
            .await;

        if existing.success {
            // delete log and return
            return self.delete_log(existing.payload.unwrap().id).await;
        }

        // return
        self.create_log(
            String::from("follow"),
            serde_json::to_string::<UserFollow>(&p).unwrap(),
        )
        .await
    }
}
