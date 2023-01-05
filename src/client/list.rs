use super::{filename, RecClient};
use crate::client::filetype;
use crate::fid::Fid;
use crate::status_check;
use chrono::prelude::*;
use fuse_mt::FileType;
use serde::Deserialize;
use serde_json::Value;
use std::convert::TryFrom;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[derive(Deserialize, Default)]
pub struct RecListEntity {
    datas: Vec<RecListData>,
}

#[allow(dead_code)]
#[derive(Deserialize, Default)]
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
    pub time_updated: SystemTime,
}

impl RecListItem {
    pub fn root() -> Self {
        Self {
            bytes: 0,
            name: "".to_string(),
            hash: None,
            fid: Fid::root(),
            ftype: FileType::Directory,
            time_updated: SystemTime::UNIX_EPOCH,
        }
    }
}

impl TryFrom<RecListData> for RecListItem {
    type Error = anyhow::Error;

    fn try_from(data: RecListData) -> Result<Self, Self::Error> {
        let time = FixedOffset::west_opt(8)
            .unwrap()
            .datetime_from_str(data.last_update_date.as_str(), "%Y-%m-%d %H:%M:%S")?
            .naive_utc();
        Ok(Self {
            bytes: match data.bytes {
                Value::String(_) => 0,
                Value::Number(i) => i.as_u64().unwrap() as usize,
                v => return Err(anyhow::Error::msg(format!("Invalid bytes field: {}", v))),
            },
            name: filename(data.name, data.file_ext),
            hash: if data.hash.is_empty() {
                None
            } else {
                Some(data.hash)
            },
            fid: Fid::from_str(data.number.as_str())?,
            ftype: filetype(data.ftype.as_str())?,
            time_updated: SystemTime::UNIX_EPOCH
                + Duration::new(
                    time.timestamp() as u64,
                    time.timestamp_subsec_nanos() as u32,
                ),
        })
    }
}

impl RecClient {
    pub fn list(&self, fid: Fid) -> anyhow::Result<Vec<RecListItem>> {
        let path = format!("folder/content/{}", fid);
        let body = self.get::<_, RecListEntity>(
            &path,
            &[
                ("disk_type", "cloud"),
                ("is_rec", "false"),
                ("category", "all"),
            ],
        )?;
        status_check!(body);
        body.entity
            .datas
            .into_iter()
            .map(RecListItem::try_from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_list() {
        let client = RecClient::default();
        let items = client.list(Fid::root()).unwrap();
        for item in items {
            if let FileType::Directory = item.ftype {
                client.list(item.fid).unwrap();
                break;
            }
        }
    }
}
