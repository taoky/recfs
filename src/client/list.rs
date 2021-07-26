use super::{filename, RecClient, RecRes};
use crate::client::filetype;
use crate::fid::Fid;
use fuse_mt::FileType;
use serde::Deserialize;
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Deserialize)]
struct RecListEntity {
    datas: Vec<RecListData>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct RecListData {
    bytes: usize,
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
    pub hash: String,
    pub fid: Fid,
    pub ftype: FileType,
}

impl TryFrom<RecListData> for RecListItem {
    type Error = anyhow::Error;

    fn try_from(data: RecListData) -> Result<Self, Self::Error> {
        Ok(Self {
            bytes: data.bytes,
            name: filename(data.name, data.file_ext),
            hash: data.hash,
            fid: Fid::from_str(data.number.as_str())?,
            ftype: filetype(data.ftype.as_str())?,
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
        let body = res.json::<RecRes<RecListEntity>>()?;
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
