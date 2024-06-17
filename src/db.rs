use dorsal::query as sqlquery;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct AppData {
    pub db: Database,
    pub http_client: awc::Client,
}

pub use dorsal::db::special::auth_db::{FullUser, RoleLevel, RoleLevelLog, UserMetadata, UserState};

pub use dorsal::db::special::log_db::{Log, LogIdentifier};
pub use dorsal::DefaultReturn;

#[derive(Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserFollow {
    pub user: String,         // the user that is following `is_following`
    pub is_following: String, // use user that `user` is following
}

#[allow(dead_code)]
pub fn deserialize_userfollow(input: String) -> UserFollow {
    serde_json::from_str::<UserFollow>(&input).unwrap()
}

// activity feed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityPost {
    pub id: String,
    pub content: String,
    pub content_html: String,
    pub author: String,
    #[serde(default)]
    /// The ID of the post this post is replying to
    pub reply: String,
    pub timestamp: u128,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PostFavoriteLog {
    /// the username of the user that favorited the post
    pub user: String,
    /// the id of the post that was favorited
    pub id: String,
}

// propss
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct PCreatePost {
    pub content: String,
    pub author: String,
    #[serde(default)]
    pub reply: String,
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

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"gup_posts\" (
                id VARCHAR(1000000),
                author VARCHAR(1000000),
                content VARCHAR(1000000),
                content_html VARCHAR(1000000),
                reply VARCHAR(1000000),
                timestamp VARCHAR(1000000)
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

    // activity

    // GET
    /// Get all user activity posts by `username`
    ///
    /// # Arguments:
    /// * `username` - [`String`]
    /// * `offset` - optional value representing the SQL fetch offset
    pub async fn get_user_activity(
        &self,
        username: String,
        offset: Option<i32>,
    ) -> DefaultReturn<Option<Vec<(ActivityPost, Vec<ActivityPost>, i32)>>> {
        let offset = if offset.is_some() { offset.unwrap() } else { 0 };

        // make sure user exists
        let existing: DefaultReturn<Option<FullUser<String>>> = self
            .get_user_by_username(username.to_owned().to_lowercase())
            .await;

        if existing.success == false {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist"),
                payload: Option::None,
            };
        }

        // check in cache
        let cached = self
            .base
            .cachedb
            .get(format!(
                "user-posts:{}:offset{}",
                username.to_lowercase(),
                offset
            ))
            .await;

        if cached.is_some() {
            // ...
            let posts =
                serde_json::from_str::<Vec<ActivityPost>>(cached.unwrap().as_str()).unwrap();

            // get replies
            let mut true_output: Vec<(ActivityPost, Vec<ActivityPost>, i32)> = Vec::new();
            for post in posts {
                let mut replies_out = Vec::new();
                let post_id = post.clone().id;

                // get replies
                let replies = &self.get_post_replies(post_id.clone(), false).await;

                if replies.payload.is_some() {
                    for reply in replies.payload.clone().unwrap() {
                        replies_out.push(reply);
                    }
                }

                // get favorites
                let favorites = &self.get_post_favorites(post_id).await;

                // ...
                true_output.push((post, replies_out, favorites.payload));
                continue;
            }

            // ...
            return DefaultReturn {
                success: true,
                message: String::from("Successfully fetched posts"),
                payload: Option::Some(true_output),
            };
        }

        // ...
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"gup_posts\" WHERE \"author\" = ? AND \"reply\" = '' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET ?"
        } else {
            "SELECT * FROM \"gup_posts\" WHERE \"author\" = $1  AND \"reply\" = '' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET $2"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
            .bind::<&String>(&username)
            .bind(offset)
            .fetch_all(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch posts"),
                payload: Option::None,
            };
        }

        // ...
        let rows = res.unwrap();
        let mut output: Vec<ActivityPost> = Vec::new();

        for row in rows {
            let row = self.base.textify_row(row).data;
            output.push(ActivityPost {
                id: row.get("id").unwrap().to_string(),
                content: row.get("content").unwrap().to_string(),
                content_html: row.get("content_html").unwrap().to_string(),
                author: row.get("author").unwrap().to_string(),
                reply: row.get("reply").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            });
        }

        // store in cache
        self.base
            .cachedb
            .set(
                format!("user-posts:{}:offset{}", username.to_lowercase(), offset),
                serde_json::to_string::<Vec<ActivityPost>>(&output).unwrap(),
            )
            .await;

        // get true output
        // we only pushed the original output to cache because replies are cached elsewhere
        let mut true_output: Vec<(ActivityPost, Vec<ActivityPost>, i32)> = Vec::new();
        for post in output {
            let mut replies_out = Vec::new();
            let post_id = post.clone().id;

            // get replies
            let replies = &self.get_post_replies(post_id.clone(), false).await;

            for reply in replies.payload.clone().unwrap() {
                replies_out.push(reply);
            }

            // get favorites
            let favorites = &self.get_post_favorites(post_id).await;

            // ...
            true_output.push((post, replies_out, favorites.payload));
            continue;
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Successfully fetched posts"),
            payload: Option::Some(true_output),
        };
    }

    /// Get all posts replying to another post by the `id` of the original post
    ///
    /// # Arguments:
    /// * `id` - post id
    /// * `run_existing_check` - if we should check that the log exists first
    pub async fn get_post_replies(
        &self,
        id: String,
        run_existing_check: bool,
    ) -> DefaultReturn<Option<Vec<ActivityPost>>> {
        // make sure post exists
        if run_existing_check != false {
            let existing: DefaultReturn<Option<ActivityPost>> =
                self.get_post_by_id(id.to_owned()).await;

            if existing.success == false {
                return DefaultReturn {
                    success: false,
                    message: String::from("Post does not exist"),
                    payload: Option::None,
                };
            }
        }

        // check in cache
        let cached = self.base.cachedb.get(format!("post-replies:{}", id)).await;

        if cached.is_some() {
            // ...
            let posts =
                serde_json::from_str::<Vec<ActivityPost>>(cached.unwrap().as_str()).unwrap();

            // ...
            return DefaultReturn {
                success: true,
                message: String::from("Successfully fetched posts"),
                payload: Option::Some(posts),
            };
        }

        // ...
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"gup_posts\" WHERE \"reply\" = ? ORDER BY \"timestamp\" DESC LIMIT 100"
        } else {
            "SELECT * FROM \"gup_posts\" WHERE \"reply\" = $1 ORDER BY \"timestamp\" DESC LIMIT 100"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query).bind::<&String>(&id).fetch_all(c).await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch replies"),
                payload: Option::None,
            };
        }

        // ...
        let rows = res.unwrap();
        let mut output: Vec<ActivityPost> = Vec::new();

        for row in rows {
            let row = self.base.textify_row(row).data;
            output.push(ActivityPost {
                id: row.get("id").unwrap().to_string(),
                content: row.get("content").unwrap().to_string(),
                content_html: row.get("content_html").unwrap().to_string(),
                author: row.get("author").unwrap().to_string(),
                reply: row.get("reply").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            });
        }

        // store in cache
        self.base
            .cachedb
            .set(
                format!("post-replies:{}", id),
                serde_json::to_string::<Vec<ActivityPost>>(&output).unwrap(),
            )
            .await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Successfully fetched replies"),
            payload: Option::Some(output),
        };
    }

    /// Get all posts replying to another post by the `id` of the original post
    ///
    /// # Arguments:
    /// * `id` - post id
    /// * `run_existing_check` - if we should check that the log exists first
    pub async fn get_post_replies_full(
        &self,
        id: String,
        run_existing_check: bool,
    ) -> DefaultReturn<Option<Vec<(ActivityPost, Vec<ActivityPost>, i32)>>> {
        // make sure post exists
        if run_existing_check != false {
            let existing: DefaultReturn<Option<ActivityPost>> =
                self.get_post_by_id(id.to_owned()).await;

            if existing.success == false {
                return DefaultReturn {
                    success: false,
                    message: String::from("Post does not exist"),
                    payload: Option::None,
                };
            }
        }

        // check in cache
        let cached = self.base.cachedb.get(format!("post-replies:{}", id)).await;

        if cached.is_some() {
            // ...
            let posts =
                serde_json::from_str::<Vec<ActivityPost>>(cached.unwrap().as_str()).unwrap();

            // get replies
            let mut true_output: Vec<(ActivityPost, Vec<ActivityPost>, i32)> = Vec::new();
            for post in posts {
                let mut replies_out = Vec::new();
                let post_id = post.clone().id;

                // get replies
                let replies = &self.get_post_replies(post_id.clone(), false).await;

                if replies.payload.is_some() {
                    for reply in replies.payload.clone().unwrap() {
                        replies_out.push(reply);
                    }
                }

                // get favorites
                let favorites = &self.get_post_favorites(post_id).await;

                // ...
                true_output.push((post, replies_out, favorites.payload));
                continue;
            }

            // ...
            return DefaultReturn {
                success: true,
                message: String::from("Successfully fetched posts"),
                payload: Option::Some(true_output),
            };
        }

        // ...
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"gup_posts\" WHERE \"reply\" = ? ORDER BY \"timestamp\" DESC LIMIT 100"
        } else {
            "SELECT * FROM \"gup_posts\" WHERE \"reply\" = $1 ORDER BY \"timestamp\" DESC LIMIT 100"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query).bind::<&String>(&id).fetch_all(c).await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Failed to fetch replies"),
                payload: Option::None,
            };
        }

        // ...
        let rows = res.unwrap();
        let mut output: Vec<ActivityPost> = Vec::new();

        for row in rows {
            let row = self.base.textify_row(row).data;
            output.push(ActivityPost {
                id: row.get("id").unwrap().to_string(),
                content: row.get("content").unwrap().to_string(),
                content_html: row.get("content_html").unwrap().to_string(),
                author: row.get("author").unwrap().to_string(),
                reply: row.get("reply").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            });
        }

        // store in cache
        self.base
            .cachedb
            .set(
                format!("post-replies:{}", id),
                serde_json::to_string::<Vec<ActivityPost>>(&output).unwrap(),
            )
            .await;

        // get true output
        // we only pushed the original output to cache because replies are cached elsewhere
        let mut true_output: Vec<(ActivityPost, Vec<ActivityPost>, i32)> = Vec::new();
        for post in output {
            let mut replies_out = Vec::new();
            let post_id = post.clone().id;

            // get replies
            let replies = &self.get_post_replies(post_id.clone(), false).await;

            for reply in replies.payload.clone().unwrap() {
                replies_out.push(reply);
            }

            // get favorites
            let favorites = &self.get_post_favorites(post_id).await;

            // ...
            true_output.push((post, replies_out, favorites.payload));
            continue;
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Successfully fetched replies"),
            payload: Option::Some(true_output),
        };
    }

    /// Get an [`ActivityPost`] by its id
    ///
    /// # Arguments:
    /// * `id` - `String` of the post's `id`
    pub async fn get_post_by_id(&self, id: String) -> DefaultReturn<Option<ActivityPost>> {
        // check in cache
        let cached = self.base.cachedb.get(format!("post:{}", id)).await;

        if cached.is_some() {
            // ...
            let post = serde_json::from_str::<ActivityPost>(cached.unwrap().as_str()).unwrap();

            // return
            return DefaultReturn {
                success: true,
                message: String::from("Post exists (cache)"),
                payload: Option::Some(post),
            };
        }

        // ...
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"gup_posts\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"gup_posts\" WHERE \"id\" = $1"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query).bind::<&String>(&id).fetch_one(c).await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from("Post does not exist"),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.base.textify_row(row).data;

        // store in cache
        let post = ActivityPost {
            id: row.get("id").unwrap().to_string(),
            content: row.get("content").unwrap().to_string(),
            content_html: row.get("content_html").unwrap().to_string(),
            author: row.get("author").unwrap().to_string(),
            reply: row.get("reply").unwrap().to_string(),
            timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        self.base
            .cachedb
            .set(
                format!("post:{}", id),
                serde_json::to_string::<ActivityPost>(&post).unwrap(),
            )
            .await;

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Post exists"),
            payload: Option::Some(post),
        };
    }

    // SET
    /// Create a new [`ActivityPost`]
    ///
    /// # Arguments:
    /// * `props` - [`PCreatePost`]
    pub async fn create_activity_post(
        &self,
        props: &mut PCreatePost,
    ) -> DefaultReturn<Option<ActivityPost>> {
        let p: &mut PCreatePost = props; // borrowed props

        // check values

        // (check length)
        if (p.content.len() < 2) | (p.content.len() > 500) {
            return DefaultReturn {
                success: false,
                message: String::from("Content is invalid"),
                payload: Option::None,
            };
        }

        // make sure author exists
        let existing: DefaultReturn<Option<FullUser<String>>> = self
            .get_user_by_username(p.author.to_owned().to_lowercase())
            .await;

        if existing.success == false {
            return DefaultReturn {
                success: false,
                message: String::from("User does not exist"),
                payload: Option::None,
            };
        }

        // create post
        let post = ActivityPost {
            id: dorsal::utility::random_id(),
            author: p.author.clone(), // posts can only be created by user accounts
            content: p.content.clone(),
            content_html: crate::markup::render(&p.content),
            reply: p.reply.clone(),
            timestamp: dorsal::utility::unix_epoch_timestamp(),
        };

        // update cache
        if p.reply.is_empty() {
            // clear author activity feed
            self.base
                .cachedb
                .remove_starting_with(format!("user-posts:{}:offset*", p.author.to_lowercase()))
                .await;
        } else {
            // get post that we're replying to (and make sure it exists)
            let replying_to = self.get_post_by_id(p.reply.clone()).await;

            if replying_to.success == false {
                return replying_to;
            }

            self.base
                .cachedb
                .remove(format!("post-replies:{}", p.reply))
                .await;
        }

        // create
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "INSERT INTO \"gup_posts\" VALUES (?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"gup_posts\" VALUES ($1, $2, $3, $4, $5, $6)"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
            .bind::<&String>(&post.id)
            .bind::<&String>(&post.author)
            .bind::<&String>(&post.content)
            .bind::<&String>(&post.content_html)
            .bind::<&String>(&post.reply)
            .bind::<&String>(&post.timestamp.to_string())
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
            message: String::from("Post created"),
            payload: Option::Some(post),
        };
    }

    /// Delete an existing [`ActivityPost`]
    ///
    /// # Arguments:
    /// * `id` - post id
    /// * `as_user` - The username of the user creating the post
    pub async fn delete_activity_post(
        &self,
        id: String,
        as_user: Option<String>,
    ) -> DefaultReturn<bool> {
        // make sure post exists
        let existing = self.get_post_by_id(id.clone()).await;

        if existing.success == false {
            return DefaultReturn {
                success: false,
                message: existing.message,
                payload: false,
            };
        }

        let existing = existing.payload.unwrap();

        if as_user.is_none() {
            return DefaultReturn {
                success: false,
                message: String::from("You do not have permission to do this."),
                payload: false,
            };
        }

        // get user
        let user = self.auth.get_user_by_username(as_user.unwrap()).await;

        match user.payload {
            Some(ua) => {
                // check if user is either activity owner OR has "ManagePosts" permission
                if (ua.user.username != existing.author)
                    && (!ua.level.permissions.contains(&"ManagePosts".to_string()))
                {
                    return DefaultReturn {
                        success: false,
                        message: String::from("You do not have permission to do this."),
                        payload: false,
                    };
                }
            }
            None => {
                return DefaultReturn {
                    success: false,
                    message: String::from("User does not exist."),
                    payload: false,
                }
            }
        }

        // delete
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "DELETE FROM \"gup_posts\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"gup_posts\" WHERE \"id\" = $1"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query).bind::<&String>(&id).execute(c).await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: false,
            };
        }

        // update cache
        self.base.cachedb.remove(format!("post:{}", id)).await;

        match existing.reply.is_empty() {
            true => {
                // clear user posts cache, this post is not a reply!
                self.base
                    .cachedb
                    .remove_starting_with(format!("user-posts:{}:offset:*", existing.author))
                    .await;
            }
            false => {
                // clear post replies
                self.base
                    .cachedb
                    .remove(format!("post-replies:{}", existing.reply))
                    .await;
            }
        }

        // return
        return DefaultReturn {
            success: true,
            message: String::from("Post deleted"),
            payload: false,
        };
    }

    // post favorites

    // GET
    /// Get the number of [`PostFavoriteLog`]s an [`ActivityPost`] has
    pub async fn get_post_favorites(&self, id: String) -> DefaultReturn<i32> {
        // get post
        let existing = self.get_post_by_id(id.clone()).await;

        if existing.success == false {
            return DefaultReturn {
                success: false,
                message: String::from("Post does not exist!"),
                payload: 0,
            };
        }

        // get favorites
        DefaultReturn {
            success: true,
            message: id.clone(),
            // favorites are stored in the "Logs" table AS WELL AS an incremented value in the cache,
            // we read the value from cache when checking the paste's favorites, but read the cache value when fetching number
            payload: self
                .base
                .cachedb
                .get(format!("social:post-favorites:{}", id))
                .await
                .unwrap_or(String::from("0"))
                .parse::<i32>()
                .unwrap(),
        }
    }

    /// Check if a user has favorited a post
    pub async fn get_user_post_favorite(
        &self,
        user: String,
        post_id: String,
        skip_existing_check: bool,
    ) -> DefaultReturn<Option<Log>> {
        // get paste
        if skip_existing_check == false {
            let existing = self.get_post_by_id(post_id.clone()).await;

            if existing.success == false {
                return DefaultReturn {
                    success: false,
                    message: String::from("Post does not exist!"),
                    payload: Option::None,
                };
            }
        }

        // ...
        let query: &str = if (self.base.db._type == "sqlite") | (self.base.db._type == "mysql") {
            "SELECT * FROM \"Logs\" WHERE \"content\" = ? AND \"logtype\" = 'post_favorite'"
        } else {
            "SELECT * FROM \"Logs\" WHERE \"content\" = $1 AND \"logtype\" = 'post_favorite'"
        };

        let c = &self.base.db.client;
        let res = sqlquery(query)
            .bind::<&String>(
                &serde_json::to_string::<PostFavoriteLog>(&PostFavoriteLog {
                    user,
                    id: post_id.clone(),
                })
                .unwrap(),
            )
            .fetch_one(c)
            .await;

        if res.is_err() {
            return DefaultReturn {
                success: false,
                message: String::from(res.err().unwrap().to_string()),
                payload: Option::None,
            };
        }

        // ...
        let row = res.unwrap();
        let row = self.base.textify_row(row).data;

        DefaultReturn {
            success: true,
            message: post_id,
            payload: Option::Some(Log {
                id: row.get("id").unwrap().to_string(),
                logtype: row.get("logtype").unwrap().to_string(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
                content: row.get("content").unwrap().to_string(),
            }),
        }
    }

    // SET
    /// Toggle a [`PostFavoriteLog`] on a [`ActivityPost`] by `user` and `post_id`
    pub async fn toggle_user_post_favorite(
        &self,
        user: String,
        post_id: String,
    ) -> DefaultReturn<Option<String>> {
        // get paste
        let existing = self.get_post_by_id(post_id.clone()).await;

        if existing.success == false {
            return DefaultReturn {
                success: false,
                message: String::from("Post does not exist!"),
                payload: Option::None,
            };
        }

        // check if user is paste owner
        let existing = existing.payload.unwrap();

        if existing.author == user {
            return DefaultReturn {
                success: false,
                message: String::from("You're the post author!"),
                payload: Option::None,
            };
        }

        // attempt to get the user's existing favorite
        let existing_favorite = self
            .get_user_post_favorite(user.clone(), post_id.clone(), true)
            .await;

        // delete existing
        if existing_favorite.success == true {
            let payload = existing_favorite.payload.unwrap();

            // decr favorites
            self.base
                .cachedb
                .decr(format!("social:post-favorites:{}", post_id.clone()))
                .await;

            // handle log
            return self.logs.delete_log(payload.id).await;
        }
        // add new
        else {
            // incr favorites
            self.base
                .cachedb
                .incr(format!("social:post-favorites:{}", post_id.clone()))
                .await;

            // handle log
            return self
                .logs
                .create_log(
                    String::from("post_favorite"),
                    serde_json::to_string::<PostFavoriteLog>(&PostFavoriteLog {
                        user,
                        id: post_id,
                    })
                    .unwrap(),
                )
                .await;
        }
    }
}
