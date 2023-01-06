use fuse_mt::FileType;
use serde_json::json;

use crate::{fid::Fid, status_check};

use super::RecClient;

#[derive(Debug)]
pub enum Operation {
    Recycle,
    Delete,
    Restore,
    Move,
    Copy,
}

impl From<Operation> for String {
    fn from(op: Operation) -> Self {
        match op {
            Operation::Recycle => "recycle".to_owned(),
            Operation::Delete => "delete".to_owned(),
            Operation::Restore => "restore".to_owned(),
            Operation::Move => "move".to_owned(),
            Operation::Copy => "copy".to_owned(),
        }
    }
}

impl RecClient {
    pub fn operation(
        &self,
        action: Operation,
        from_id: Fid,
        from_type: FileType,
        dst_id: Option<String>,
    ) -> anyhow::Result<()> {
        let action: String = action.into();
        let dst_id = dst_id.unwrap_or_default();
        let resp = self.post::<_, serde_json::Value>(
            "operationFileOrFolder",
            &json!({
                "action": action,
                "disk_type": "cloud",
                "files_list": [{"number": from_id.to_string(), "type": match from_type {
                    FileType::Directory => "folder",
                    FileType::RegularFile => "file",
                    _ => unreachable!(),
                }}],
                "number": if dst_id == "B_0".to_string() { "0".to_string() } else { dst_id }
            }),
        )?;
        status_check!(resp);
        Ok(())
    }

    pub fn rename(&self, id: Fid, new_name: String, filetype: FileType) -> anyhow::Result<()> {
        let resp = self.post::<_, serde_json::Value>(
            "rename",
            &json!({
                "name": new_name,
                "number": id.to_string(),
                "type": match filetype {
                    FileType::Directory => "folder",
                    FileType::RegularFile => "file",
                    _ => unreachable!(),
                }
            }),
        )?;
        status_check!(resp);
        Ok(())
    }
}
