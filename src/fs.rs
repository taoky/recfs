use crate::client::auth::RecAuth;
use crate::client::list::RecListItem;
use crate::client::RecClient;
use crate::fid::Fid;
use crate::fidmap::FidMap;
use fuse_mt::{
    DirectoryEntry, FileAttr, FilesystemMT, RequestInfo, ResultEntry, ResultOpen, ResultReaddir,
};
use std::borrow::{Borrow, BorrowMut};
use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

pub struct RecFs {
    client: RecClient,
    fid_map: Arc<RwLock<FidMap>>,
}

impl RecFs {
    pub fn new() -> Self {
        let mut client = RecClient::default();
        let mut auth = RecAuth::default();

        auth.login(&client, "114514".to_owned(), "1919810".to_owned()).unwrap();
        client.set_auth(auth);
        
        Self {
            client,
            fid_map: Arc::new(RwLock::new(FidMap::new())),
        }
    }
}

impl FilesystemMT for RecFs {
    fn getattr(&self, _req: RequestInfo, path: &Path, fh: Option<u64>) -> ResultEntry {
        let (fid, parent_fid) = if let Some(fh) = fh {
            self.get_fid_with_parent(fh)?
        } else {
            self.req_fid(path)?
        };
        let item = self.req_item(fid, parent_fid)?;
        let attr = FileAttr {
            size: item.bytes as u64,
            blocks: 1,
            atime: item.time_updated,
            mtime: item.time_updated,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind: item.ftype,
            perm: (libc::S_IRUSR | libc::S_IWUSR) as u16,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        Ok((Duration::new(1, 0), attr))
    }

    fn opendir(&self, _req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        let (fid, parent_fid) = self.req_fid(path)?;
        Ok((
            self.fid_map
                .write()
                .unwrap()
                .borrow_mut()
                .set(fid, parent_fid),
            0,
        ))
    }

    fn readdir(&self, _req: RequestInfo, _path: &Path, fh: u64) -> ResultReaddir {
        let fid = self.get_fid(fh)?;
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

impl RecFs {
    fn req_fid(&self, path: &Path) -> Result<(Fid, Option<Fid>), libc::c_int> {
        let mut parent_fid = None;
        let mut fid = Fid::root();
        for c in path.components().skip(1) {
            let items = self.client.list(fid).map_err(|_| libc::ENOENT)?;
            let s = c.as_os_str().to_string_lossy();
            match items.iter().find(|i| i.name == s) {
                Some(item) => {
                    parent_fid = Some(fid);
                    fid = item.fid;
                }
                None => return Err(libc::ENOENT),
            }
        }
        Ok((fid, parent_fid))
    }

    fn get_fid(&self, fh: u64) -> Result<Fid, libc::c_int> {
        self.fid_map
            .read()
            .unwrap()
            .borrow()
            .get(fh)
            .ok_or(libc::EBADF)
    }

    fn get_fid_with_parent(&self, fh: u64) -> Result<(Fid, Option<Fid>), libc::c_int> {
        let map = self.fid_map.read().unwrap();
        let fid = map.borrow().get(fh).ok_or(libc::EBADF)?;
        let parent_fid = map.borrow().get_parent(fid).unwrap();
        Ok((fid, parent_fid))
    }

    fn req_item(&self, fid: Fid, parent_fid: Option<Fid>) -> Result<RecListItem, libc::c_int> {
        if let Some(parent_fid) = parent_fid {
            let items = self.client.list(parent_fid).map_err(|_| libc::ENOENT)?;
            Ok(items.into_iter().find(|i| i.fid == fid).unwrap())
        } else {
            Ok(RecListItem::root())
        }
    }
}
