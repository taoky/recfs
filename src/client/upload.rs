use std::{io::Read, path::Path};

use log::{info, warn};
use serde::Deserialize;
use serde_json::json;

use crate::{fid::Fid, status_check};

use super::RecClient;

#[derive(Deserialize)]
#[allow(dead_code)]
struct RecUploadParam {
    key: String,
    request_type: String,
    value: String,
}

type RecUploadParams = Vec<Vec<RecUploadParam>>;

impl RecClient {
    pub fn upload(
        &self,
        parent_fid: Fid,
        file_path: &Path,
        file_name: String,
    ) -> anyhow::Result<()> {
        let filesize = file_path.metadata()?.len();
        let resp = self.get::<_, serde_json::Value>(
            &format!("file/{}", parent_fid),
            &[
                ("file_name", file_name),
                ("byte", filesize.to_string()),
                ("storage", "moss".to_owned()),
                ("disk_type", "cloud".to_owned()),
            ],
        )?;
        status_check!(resp);
        let resp = resp.entity;
        let upload_token: String = serde_json::from_value(resp["upload_token"].clone())?;

        let upload_chunk_size: String = serde_json::from_value(resp["upload_chunk_size"].clone())?;
        let upload_chunk_size: usize = upload_chunk_size.parse()?;

        let upload_params: RecUploadParams = serde_json::from_value(resp["upload_params"].clone())?;

        let mut file = std::fs::File::open(file_path)?;

        for (idx, i) in upload_params.into_iter().enumerate() {
            let mut buffer: Vec<u8> = vec![0; upload_chunk_size as usize];
            // TODO: ignore read() amount for now
            let _ignored = file.read(&mut buffer)?;
            let upload_url = &i[1].value;
            let upload_method = &i[2].value;
            if upload_method != "PUT" {
                return Err(anyhow::anyhow!(
                    "Unsupported upload method: {}",
                    upload_method
                ));
            }
            if let Err(e) = self.put_upload(upload_url, buffer) {
                warn!("Upload part {} err with {}", idx, e);
                return Err(e);
            } else {
                info!("Upload part {} ok", idx);
            }
        }

        let resp = self.post::<_, serde_json::Value>(
            "file/complete",
            &json!({ "upload_token": upload_token }),
        )?;
        status_check!(resp);

        Ok(())
    }
}
