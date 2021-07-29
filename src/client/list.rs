use super::{filename, RecClient, RecRes};
use crate::client::filetype;
use crate::fid::Fid;
use chrono::prelude::*;
use fuse_mt::FileType;
use serde::Deserialize;
use serde_json::Value;
use std::convert::TryFrom;
use std::str::FromStr;
use time::Timespec;

#[derive(Deserialize)]
struct RecListEntity {
    datas: Vec<RecListData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct RecListData {
    bytes: Value,
    file_ext: String,
    file_type: String,
    hash: String,
    last_update_date: String,
    name: String,
    number: String,
    parent_number: String,
    #[serde(rename = "type")]
    ftype: String,
}

pub struct RecListItem {
    pub bytes: usize,
    pub name: String,
    pub hash: Option<String>,
    pub fid: Fid,
    pub ftype: FileType,
    pub time_updated: Timespec,
}

impl RecListItem {
    pub fn root() -> Self {
        Self {
            bytes: 0,
            name: "".to_string(),
            hash: None,
            fid: Fid::root(),
            ftype: FileType::Directory,
            time_updated: Timespec::new(0, 0),
        }
    }
}

impl TryFrom<RecListData> for RecListItem {
    type Error = anyhow::Error;

    fn try_from(data: RecListData) -> Result<Self, Self::Error> {
        let time = FixedOffset::west(8)
            .datetime_from_str(data.last_update_date.as_str(), "%Y-%m-%d %H:%M:%S")?
            .naive_utc();
        Ok(Self {
            bytes: match data.bytes {
                Value::String(_) => 0,
                Value::Number(i) => i.as_u64().unwrap() as usize,
                v => return Err(anyhow::Error::msg(format!("Invalid bytes field: {}", v))),
            },
            name: filename(data.name, data.file_ext),
            hash: if data.hash == "" {
                None
            } else {
                Some(data.hash)
            },
            fid: Fid::from_str(data.number.as_str())?,
            ftype: filetype(data.ftype.as_str())?,
            time_updated: Timespec::new(time.timestamp(), time.timestamp_subsec_nanos() as i32),
        })
    }
}

impl RecClient {
    pub fn list(&self, fid: Fid) -> anyhow::Result<Vec<RecListItem>> {
        let url = format!("https://recapi.ustc.edu.cn/api/v2/folder/content/{}?disk_type=cloud&is_rec=false&category=all", fid);
        let res = self
            .client
            .get(url)
            .header("x-auth-token", self.auth_token.as_str())
            .send()?;
        let body = serde_json::from_str::<RecRes<RecListEntity>>(
            res.text()?.trim_start_matches("\u{feff}"),
        )?;
        if body.status_code != 200 {
            Err(anyhow::Error::msg(format!(
                "Status code {}",
                body.status_code
            )))
        } else {
            body.entity
                .datas
                .into_iter()
                .map(|d| RecListItem::try_from(d))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_list() {
        let client = RecClient::new(env::var("AUTH_TOKEN").unwrap().to_string());
        let items = client.list(Fid::root()).unwrap();
        for item in items {
            if let FileType::Directory = item.ftype {
                client.list(item.fid).unwrap();
                break;
            }
        }
    }
}
