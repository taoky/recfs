use serde_json::json;

use crate::{fid::Fid, status_check};

use super::RecClient;

impl RecClient {
    pub fn get_download_url(&self, fid: Fid) -> anyhow::Result<String> {
        let resp = self.post::<_, serde_json::Value>(
            "download",
            &json!({
                "files_list": [fid.to_string()]
            }),
        )?;
        status_check!(resp);
        let url = resp.entity[fid.to_string()]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to get download url for fid: {}", fid))?;
        Ok(url.to_owned())
    }
}
