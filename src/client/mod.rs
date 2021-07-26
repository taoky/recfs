mod list;

use reqwest::blocking::Client;
use serde::Deserialize;

pub struct RecClient {
    auth_token: String,
    client: Client,
}

#[derive(Deserialize)]
struct RecRes<T> {
    entity: T,
    status_code: i32,
}

impl RecClient {
    pub fn new(auth_token: String) -> Self {
        Self {
            auth_token,
            client: Client::new(),
        }
    }
}

pub fn filename(name: String, ext: String) -> String {
    if ext == "" {
        name
    } else {
        format!("{}.{}", name, ext)
    }
}
