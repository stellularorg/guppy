pub struct BaseTemplate {
    pub info: String,
    pub auth_state: bool,
    pub bundlrs: String,
    pub puffer: String,
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
        bundlrs: std::env::var("BUNDLRS_ROOT").unwrap(),
        puffer: std::env::var("PUFFER_ROOT").unwrap(),
        body_embed,
    }
}
