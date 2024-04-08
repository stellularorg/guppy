use dorsal::query as sqlquery;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppData {
    pub db: Database,
    pub http_client: awc::Client,
}

pub use dorsal::db::special::auth_db::{
    FullUser, RoleLevel, RoleLevelLog, UserMetadata, UserState,
};

pub use dorsal::db::special::log_db::{Log, LogIdentifier};
pub use dorsal::DefaultReturn;

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserFollow {
    pub user: String,         // the user that is following `is_following`
    pub is_following: String, // use user that `user` is following
}

#[derive(Default, PartialEq, Clone, Serialize, Deserialize)]
pub struct Board<M> {
    // selectors
    pub name: String,
    // dates
    pub timestamp: u128,
    // ...
    pub metadata: M,
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardMetadata {
    pub owner: String,                      // username of owner
    pub is_private: String, // if the homepage of the board is shown to other users (not owner)
    pub allow_anonymous: Option<String>, // if anonymous users can post
    pub allow_open_posting: Option<String>, // if all users can post on the board (not just owner)
    pub topic_required: Option<String>, // if posts are required to include a topic value
    pub about: Option<String>, // welcome message
    pub tags: Option<String>, // SPACE separated list of tags that identify the board for searches
                            // TODO: we could likely export a list of "valid" tags at some point in the future
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardPostLog {
    pub author: String, // username of owner
    pub content: String,
    pub content_html: String,
    pub topic: Option<String>, // post topic, content is hidden unless expanded if provided
    pub board: String,         // name of board the post is located in
    pub is_hidden: bool,       // if the post is hidden in the feed (does nothing right now)
    pub reply: Option<String>, // the ID of the post we're replying to
    pub pinned: Option<bool>,  // pin the post to the top of the board feed
    pub replies: Option<usize>, // not really managed in the log, just used to show the number of replies this post has
    pub tags: Option<String>,   // same as board tags, just for posts specifically
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardIdentifier {
    pub name: String,
    pub tags: String,
}

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
// Takes the place of "about" in BoardMetadata, identifies a board as a user mail stream
pub struct UserMailStreamIdentifier {
    pub _is_user_mail_stream: bool, // always going to be true ... cannot be edited into board about ANYWHERE
    pub user1: String,              // username of first user
    pub user2: String,              // username of second user
}

#[allow(dead_code)]
pub fn deserialize_userfollow(input: String) -> UserFollow {
    serde_json::from_str::<UserFollow>(&input).unwrap()
}

// server
#[derive(Clone)]
pub struct Database {
    pub base: dorsal::StarterDatabase,
    pub auth: dorsal::AuthDatabase,
    pub logs: dorsal::LogDatabase,
}

impl Database {
    pub async fn new(opts: dorsal::DatabaseOpts) -> Database {
        let db = dorsal::StarterDatabase::new(opts).await;

        Database {
            base: db.clone(),
            auth: dorsal::AuthDatabase { base: db.clone() },
            logs: dorsal::LogDatabase { base: db },
        }
    }

    pub async fn init(&self) {
        let c = &self.base.db.client;

        let _ = sqlquery(
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

        let _ = sqlquery(
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
        self.auth.get_user_by_hashed(hashed).await
    }

    /// Get a user by their unhashed ID (hashes ID and then calls [`Database::get_user_by_hashed()`])
    ///
    /// Calls [`Database::get_user_by_unhashed_st()`] if user is invalid.
    ///
    /// # Arguments:
    /// * `unhashed` - `String` of the user's unhashed ID
    pub async fn get_user_by_unhashed(
        &self,
        unhashed: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        self.auth.get_user_by_unhashed(unhashed).await
    }

    /// Get a user by their unhashed secondary token
    ///
    /// # Arguments:
    /// * `unhashed` - `String` of the user's unhashed secondary token
    pub async fn get_user_by_unhashed_st(
        &self,
        unhashed: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        self.auth.get_user_by_unhashed_st(unhashed).await
    }

    /// Get a user by their username
    ///
    /// # Arguments:
    /// * `username` - `String` of the user's username
    pub async fn get_user_by_username(
        &self,
        username: String,
    ) -> DefaultReturn<Option<FullUser<String>>> {
        self.auth.get_user_by_username(username).await
    }

    /// Get a [`RoleLevel`] by its `name`
    ///
    /// # Arguments:
    /// * `name` - `String` of the level's role name
    pub async fn get_level_by_role(&self, name: String) -> DefaultReturn<RoleLevelLog> {
        self.auth.get_level_by_role(name).await
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "INSERT INTO \"Users\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"Users\" VALUES ($1, $2, $3, $4, $5)"
        };

        let user_id_unhashed: String = dorsal::utility::uuid();
        let user_id_hashed: String = dorsal::utility::hash(user_id_unhashed.clone());
        let timestamp = dorsal::utility::unix_epoch_timestamp().to_string();

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "UPDATE \"Users\" SET \"metadata\" = ? WHERE \"username\" = ?"
        } else {
            "UPDATE \"Users\" SET (\"metadata\") = ($1) WHERE \"username\" = $2"
        };

        let c = &self.base.db.client;
        let meta = &serde_json::to_string(&metadata).unwrap();
        let res = sqlquery(query)
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
        let existing_in_cache = self.base.cachedb.get(format!("user:{}", name)).await;

        if existing_in_cache.is_some() {
            let mut user =
                serde_json::from_str::<UserState<String>>(&existing_in_cache.unwrap()).unwrap();
            user.metadata = meta.to_string(); // update metadata

            // update cache
            self.base
                .cachedb
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "UPDATE \"Users\" SET \"role\" = ? WHERE \"username\" = ?"
        } else {
            "UPDATE \"Users\" SET (\"role\") = ($1) WHERE \"username\" = $2"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
        let existing_in_cache = self.base.cachedb.get(format!("user:{}", name)).await;

        if existing_in_cache.is_some() {
            let mut user =
                serde_json::from_str::<UserState<String>>(&existing_in_cache.unwrap()).unwrap();
            user.role = String::from("banned"); // update role

            // update cache
            self.base
                .cachedb
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow'"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
        let row = self.base.textify_row(row).data;

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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET ?"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET $2"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
            let row = self.base.textify_row(row).data;
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET ?"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET $2"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
            let row = self.base.textify_row(row).data;
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow'"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE ? AND \"logtype\" = 'follow'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" LIKE $1 AND \"logtype\" = 'follow'"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
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
            return self.logs.delete_log(existing.payload.unwrap().id).await;
        }

        // return
        self.logs
            .create_log(
                String::from("follow"),
                serde_json::to_string::<UserFollow>(&p).unwrap(),
            )
            .await
    }

    // mail streams

    // GET
    /// Get a [`UserMailStreamIdentifier`] [`Board`] by its users
    ///
    /// # Arguments:
    /// * `props` - [`UserMailStreamIdentifier`]
    pub async fn get_mail_stream_by_users(
        &self,
        props: UserMailStreamIdentifier,
    ) -> DefaultReturn<Option<Board<String>>> {
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Boards\" WHERE \"metadata\" LIKE ?"
        } else {
            "SELECT * FROM \"Boards\" WHERE \"metadata\" LIKE $1"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
            .bind::<&String>(&format!(
                "%\"about\":\"{}\"%",
                if &self.base.db._type == "mysql" {
                    serde_json::to_string::<UserMailStreamIdentifier>(&props)
                        .unwrap()
                        .replace("\"", "\\\\\"")
                } else {
                    serde_json::to_string::<UserMailStreamIdentifier>(&props)
                        .unwrap()
                        .replace("\"", "\\\"")
                }
            ))
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Board does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.base.textify_row(row).data;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Board exists"),
            payload: Option::Some(Board {
                name: row.get("name").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
                metadata: row.get("metadata").unwrap().to_string(),
            }),
        };
    }

    /// Get all [`UserMailStreamIdentifier`] [`Board`] by a single participating user
    ///
    /// # Arguments:
    /// * `user` - username of the user
    /// * `offset` - optional value representing the SQL fetch offset
    pub async fn get_user_mail_streams(
        &self,
        user: String,
        offset: Option<i32>,
    ) -> DefaultReturn<Vec<BoardIdentifier>> {
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Boards\" WHERE \"metadata\" LIKE ? OR \"metadata\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET ?"
        } else {
            "SELECT * FROM \"Boards\" WHERE \"metadata\" LIKE $1 OR \"metadata\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET $3"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
            .bind::<String>(if &self.base.db._type == "mysql" {
                format!("%\\\\\"user1\\\\\":\\\\\"{}\\\\\"%", user)
            } else {
                format!("%\\\"user1\\\":\\\"{}\\\"%", user)
            })
            .bind::<String>(if &self.base.db._type == "mysql" {
                format!("%\\\\\"user2\\\\\":\\\\\"{}\\\\\"%", user)
            } else {
                format!("%\\\"user2\\\":\\\"{}\\\"%", user)
            })
            .bind(if offset.is_some() { offset.unwrap() } else { 0 })
            .fetch_all(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Boards do not exist"),
                payload: Vec::new(),
            };
        }

        // ...
        let rows = res.unwrap();
        let mut output: Vec<BoardIdentifier> = Vec::new();

        for row in rows {
            let row = self.base.textify_row(row).data;

            let metadata =
                serde_json::from_str::<BoardMetadata>(row.get("metadata").unwrap()).unwrap();

            let mailstream =
                serde_json::from_str::<UserMailStreamIdentifier>(&metadata.about.unwrap()).unwrap();

            output.push(BoardIdentifier {
                name: row.get("name").unwrap().to_string(),
                // we're going to use tags to store the name of the other user
                tags: if user == mailstream.user1 {
                    mailstream.user2
                } else {
                    mailstream.user1
                },
            });
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Boards exists"),
            payload: output,
        };
    }

    // SET
    /// Create a new [`Board`] given various [properties](UserMailStreamIdentifier)
    ///
    /// # Arguments:
    /// * `props` - [`UserMailStreamIdentifier`]
    pub async fn create_mail_stream(
        &self,
        props: &mut UserMailStreamIdentifier,
    ) -> DefaultReturn<Option<Board<String>>> {
        let p: &mut UserMailStreamIdentifier = props; // borrowed props

        // create default metadata
        let metadata: BoardMetadata = BoardMetadata {
            owner: String::new(),
            is_private: String::from("yes"),
            allow_anonymous: Option::Some(String::from("no")),
            allow_open_posting: Option::Some(String::from("yes")),
            topic_required: Option::Some(String::from("no")), // change to "yes" for a more inbox-like thing
            about: Option::Some(
                serde_json::to_string::<UserMailStreamIdentifier>(&p.clone()).unwrap(),
            ),
            tags: Option::None,
        };

        // check values

        // make sure board does not exist
        let existing: DefaultReturn<Option<Board<String>>> =
            self.get_mail_stream_by_users(props.to_owned()).await;

        if existing.success {
            return DefaultReturn {
                success: true, // return true so client still redirects
                message: String::from("Board already exists!"),
                payload: existing.payload, // return existing board if it does
            };
        }

        // make sure board does not exist
        let existing: DefaultReturn<Option<Board<String>>> = self
            .get_mail_stream_by_users(UserMailStreamIdentifier {
                _is_user_mail_stream: true,
                user1: props.user2.clone(),
                user2: props.user1.clone(),
            })
            .await;

        if existing.success {
            return DefaultReturn {
                success: true, // return true so client still redirects
                message: String::from("Board already exists!"),
                payload: existing.payload, // return existing board if it does
            };
        }

        // make sure other user exists (user1 should be the current user)
        let existing: DefaultReturn<Option<FullUser<String>>> =
            self.get_user_by_username(props.user2.clone()).await;

        if !existing.success {
            return DefaultReturn {
                success: false,
                message: String::from("Other user is invalid"),
                payload: Option::None,
            };
        }

        // check user permissions
        let user2_metadata =
            serde_json::from_str::<UserMetadata>(&existing.payload.unwrap().user.metadata).unwrap();

        if user2_metadata.allow_mail.is_some()
            && user2_metadata.allow_mail.as_ref().unwrap() == "no"
        {
            return DefaultReturn {
                success: false,
                message: String::from("User is not accepting mail"),
                payload: Option::None,
            };
        }

        // create board
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "INSERT INTO \"Boards\" VALUES (?, ?, ?)"
        } else {
            "INSERT INTO \"Boards\" VALUES ($1, $2, $3)"
        };

        let c = &self.base.db.client;
        let p: &mut Board<String> = &mut Board {
            name: format!("inbox-{}", dorsal::utility::random_id()),
            timestamp: dorsal::utility::unix_epoch_timestamp(),
            metadata: String::new(),
        };

        let res = sqlquery(query)
            .bind::<&String>(&p.name)
            .bind::<&String>(&p.timestamp.to_string())
            .bind::<&String>(&serde_json::to_string(&metadata).unwrap())
            .execute(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: res.err().unwrap().to_string(),
                payload: Option::None,
            };
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Created board"),
            payload: Option::Some(p.to_owned()),
        };
    }
}
