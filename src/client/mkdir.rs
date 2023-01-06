use serde_json::json;

use crate::{fid::Fid, status_check};

use super::RecClient;

impl RecClient {
    pub fn mkdir(&self, parent: Fid, name: String) -> anyhow::Result<()> {
        let resp = self.post::<_, serde_json::Value>(
            "folder/tree",
            &json!({
                "disk_type": "cloud",
                "number": parent.to_string(),
                "paramslist": [name]
            }),
        )?;
        status_check!(resp);
        Ok(())
    }
}
