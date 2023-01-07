use std::{path::Path, ffi::OsStr, io::Read};

use log::info;
use serde::Deserialize;
use serde_json::json;

use crate::{fid::Fid, status_check};

use super::RecClient;

#[derive(Deserialize)]
struct RecUploadParam {
    key: String,
    request_type: String,
    value: String
}

#[derive(Deserialize)]
struct RecUploadParams {
    upload_params: Vec<Vec<RecUploadParam>>,
}

impl RecClient {
    pub fn upload(&self, fid: Fid, file_path: &Path) -> anyhow::Result<()> {
        let filename = file_path.file_name().and_then(OsStr::to_str).ok_or(anyhow::anyhow!(
            "Failed to get filename from path: {}",
            file_path.display()
        ))?;
        let filesize = file_path.metadata()?.len();
        let resp = self.post::<_, serde_json::Value>(
            &format!("file/{}", fid.to_string()),
            &json!({
                "file_name": filename,
                "byte": filesize,
                "storage": "moss",
                "disk_type": "cloud"
            }),
        )?;
        status_check!(resp);
        let resp = resp.entity;
        let upload_token: String = serde_json::from_value(resp["upload_token"].clone())?;

        let upload_chunk_size: String = serde_json::from_value(resp["upload_chunk_size"].clone())?;
        let upload_chunk_size: usize = upload_chunk_size.parse()?;

        let upload_params: RecUploadParams = serde_json::from_value(resp["upload_params"].clone())?;

        let mut file = std::fs::File::open(file_path)?;

        for (idx, i) in upload_params.upload_params.into_iter().enumerate() {
            let mut buffer: Vec<u8> = vec![0; upload_chunk_size as usize];
            file.read(&mut buffer)?;
            let upload_url = &i[1].value;
            let upload_method = &i[2].value;
            if upload_method != "PUT" {
                return Err(anyhow::anyhow!("Unsupported upload method: {}", upload_method));
            }
            let resp = self.put::<serde_json::Value>(upload_url, buffer)?;
            info!("Upload part {} gets response {:?}", idx, resp);
            status_check!(resp);
        }

        let resp = self.post::<_, serde_json::Value>("file/complete", &json!({
            "upload_token": upload_token
        }))?;
        status_check!(resp);

        Ok(())
    }
}
