use crate::client::RecClient;
use crate::fid::Fid;
use crate::fidmap::FidMap;
use fuse_mt::{DirectoryEntry, FilesystemMT, RequestInfo, ResultOpen, ResultReaddir};
use std::borrow::{Borrow, BorrowMut};
use std::ffi::OsString;
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

    fn readdir(&self, _req: RequestInfo, _path: &Path, fh: u64) -> ResultReaddir {
        let fid = self
            .fid_map
            .read()
            .unwrap()
            .borrow()
            .get(fh)
            .ok_or(libc::EBADF)?;
        let items = self.client.list(fid).map_err(|_| libc::ENOENT)?;
        Ok(items
            .into_iter()
            .map(|i| DirectoryEntry {
                name: OsString::from(i.name),
                kind: i.ftype,
            })
            .collect())
    }
}
