use crate::client::RecClient;
use crate::fid::Fid;
use crate::fidmap::FidMap;
use fuse_mt::{FilesystemMT, RequestInfo, ResultOpen};
use std::borrow::BorrowMut;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub struct RecFs {
    client: RecClient,
    fid_map: Arc<RwLock<FidMap>>,
}

impl RecFs {
    pub fn new(auth_token: String) -> Self {
        Self {
            client: RecClient::new(auth_token),
            fid_map: Arc::new(RwLock::new(FidMap::new())),
        }
    }
}

impl FilesystemMT for RecFs {
    fn opendir(&self, _req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        let mut fid = Fid::root();
        for c in path.canonicalize().unwrap().components().skip(1) {
            let items = self.client.list(fid).map_err(|_| libc::ENOENT)?;
            let s = c.as_os_str().to_string_lossy();
            match items.iter().find(|i| i.name == s) {
                Some(item) => fid = item.fid,
                None => return Err(libc::ENOENT),
            }
        }
        Ok((self.fid_map.write().unwrap().borrow_mut().set(fid), 0))
    }
}
