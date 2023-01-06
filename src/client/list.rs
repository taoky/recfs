use super::{filename, RecClient};
use crate::client::filetype;
use crate::fid::Fid;
use crate::status_check;
use chrono::prelude::*;
use fuse_mt::FileType;
use log::debug;
use serde::Deserialize;
use serde_json::Value;
use std::convert::TryFrom;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

#[derive(Deserialize, Default, Debug)]
pub struct RecListEntity {
    datas: Vec<RecListData>,
}

#[allow(dead_code)]
#[derive(Deserialize, Default, Debug)]
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

#[derive(Debug, Clone)]
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
        let path = if fid.to_string() == "B_0" {
            "folder/content/0".to_owned()
        } else {
            format!("folder/content/{}", fid)
        };
        let body = self.get::<_, RecListEntity>(
            &path,
            &[
                (
                    "disk_type",
                    match fid.to_string().as_str() {
                        "B_0" => "backup",
                        "R_0" => "recycle",
                        _ => "cloud",
                    },
                ),
                ("is_rec", "false"),
                ("category", "all"),
            ],
        )?;
        debug!("list() body: {:?}", body);
        status_check!(body);
        let mut items = body
            .entity
            .datas
            .into_iter()
            .map(RecListItem::try_from)
            .collect::<anyhow::Result<Vec<RecListItem>>>()?;
        if fid == Fid::root() {
            items.push(RecListItem {
                bytes: 0,
                name: "?Backup".to_string(),
                hash: None,
                fid: "B_0".to_string().into(),
                ftype: FileType::Directory,
                time_updated: SystemTime::UNIX_EPOCH,
            });
            items.push(RecListItem {
                bytes: 0,
                name: "?Recycle".to_string(),
                hash: None,
                fid: "R_0".to_string().into(),
                ftype: FileType::Directory,
                time_updated: SystemTime::UNIX_EPOCH,
            });
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use crate::client::auth::RecAuth;

    use super::*;

    #[test]
    fn test_list() {
        let mut client = RecClient::default();
        let mut auth = RecAuth::default();
        auth.try_keyring().unwrap();
        client.set_auth(auth);

        let items = client.list(Fid::root()).unwrap();
        for item in items {
            if let FileType::Directory = item.ftype {
                client.list(item.fid).unwrap();
                break;
            }
        }
    }
}
