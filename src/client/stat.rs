use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::{client::EmptyQuery, status_check};

use super::RecClient;

#[serde_as]
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct RecUserInfo {
    user_type: i32,
    user_group_id: i32,
    user_number: String,
    gid: String,
    username: String,
    name: String,
    email: String,
    mobile: String,
    profile: String,
    gender: i32,
    avatar: String,
    #[serde_as(as = "DisplayFromStr")]
    pub total_space: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub used_space: u64,
    user_file_count: i32,
    user_share_count: i32,
    user_group_count: i32,
    is_backup_file: bool,
}

impl RecClient {
    pub fn stat(&self) -> anyhow::Result<RecUserInfo> {
        let body = self.get::<EmptyQuery, RecUserInfo>("userinfo", &[])?;
        status_check!(body);
        Ok(body.entity)
    }
}
