use crate::cache::Cache;
use crate::client::auth::RecAuth;
use crate::client::list::RecListItem;
use crate::client::RecClient;
use crate::fid::Fid;
use crate::fidmap::{FidCachedList, FidMap};
use fuse_mt::{
    DirectoryEntry, FileAttr, FileType, FilesystemMT, RequestInfo, ResultEntry, ResultOpen,
    ResultReaddir, ResultStatfs, Statfs,
};
use libc::O_RDONLY;
use log::{debug, info, warn};
use std::borrow::{Borrow, BorrowMut};
use std::ffi::OsString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

pub struct RecFs {
    client: RecClient,
    fid_map: Arc<RwLock<FidMap>>,
    disk_cache: Cache,
}

const BLOCK_SIZE: u32 = 512;

impl RecFs {
    pub fn new() -> Self {
        let mut client = RecClient::default();
        let mut auth = RecAuth::default();

        if let Err(e) = auth.try_keyring() {
            info!("Failed to get auth from keyring: {}", e);
            info!("Try interactive login...");
            let (username, password) = RecAuth::interactive();
            auth.login(&client, username, password).unwrap();
        }
        client.set_auth(auth);

        Self {
            client,
            fid_map: Arc::new(RwLock::new(FidMap::new())),
            disk_cache: Cache::default(),
        }
    }
}

impl From<RecListItem> for FileAttr {
    fn from(item: RecListItem) -> Self {
        FileAttr {
            size: item.bytes as u64,
            blocks: item.bytes as u64 / BLOCK_SIZE as u64,
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
        }
    }
}

impl FilesystemMT for RecFs {
    fn getattr(&self, _req: RequestInfo, path: &Path, fh: Option<u64>) -> ResultEntry {
        let (fid, parent) = if let Some(fh) = fh {
            self.get_fid_with_parent(fh)?
        } else {
            self.req_fid(path)?
        };
        let item = self.get_item(fid, parent)?;
        let attr: FileAttr = item.into();
        Ok((Duration::new(1, 0), attr))
    }

    fn opendir(&self, _req: RequestInfo, path: &Path, _flags: u32) -> ResultOpen {
        let (fid, parent) = self.req_fid(path)?;
        debug!("opendir(): {} {:?}", fid, parent);
        let item = self.get_item(fid.clone(), parent.clone())?;
        info!("opendir(): Dir opened: {:?}", item);
        if item.ftype != FileType::Directory {
            return Err(libc::ENOTDIR);
        }
        Ok((
            self.fid_map
                .write()
                .unwrap()
                .borrow_mut()
                .set_fh(&fid, parent.as_ref(), None),
            0,
        ))
    }

    // readdir only reads the "cache" generated by opendir()
    fn readdir(&self, _req: RequestInfo, _path: &Path, fh: u64) -> ResultReaddir {
        // let fid = self.get_fid(fh)?;
        // let items = self.client.list(fid.clone()).map_err(|_| libc::ENOENT)?;
        let (fid, _parent) = self.get_fid_with_parent(fh)?;
        let listing = self.get_listing(fid)?;
        Ok(listing
            .children
            .ok_or(libc::ENOTDIR)?
            .iter()
            .map(|i| DirectoryEntry {
                name: OsString::from(i.name.clone()),
                kind: i.ftype,
            })
            .collect())
    }

    fn statfs(&self, _req: RequestInfo, _path: &Path) -> ResultStatfs {
        let userinfo = self.client.stat().map_err(|_| libc::ENOENT)?;
        info!("statfs: {:?}", userinfo);
        Ok(Statfs {
            blocks: userinfo.total_space / BLOCK_SIZE as u64,
            bfree: (userinfo.total_space - userinfo.used_space) / BLOCK_SIZE as u64,
            bavail: (userinfo.total_space - userinfo.used_space) / BLOCK_SIZE as u64,
            files: 0,
            ffree: 0,
            bsize: BLOCK_SIZE,
            namelen: 255, // I also don't know how long a file in rec can be
            frsize: BLOCK_SIZE,
        })
    }

    fn open(&self, _req: RequestInfo, path: &Path, flags: u32) -> ResultOpen {
        let (fid, parent) = self.req_fid(path)?;
        let item = self.get_item(fid.clone(), parent.clone())?;
        if item.ftype != FileType::RegularFile {
            return Err(libc::EISDIR);
        }
        if (flags | O_RDONLY as u32) == 0 {
            // write mode unimplemented
            return Err(libc::ENOSYS);
        }
        Ok((
            self.fid_map
                .write()
                .unwrap()
                .borrow_mut()
                .set_fh(&fid, parent.as_ref(), None),
            flags,
        ))
    }

    fn read(
        &self,
        _req: RequestInfo,
        _path: &Path,
        fh: u64,
        offset: u64,
        size: u32,
        callback: impl FnOnce(fuse_mt::ResultSlice<'_>) -> fuse_mt::CallbackResult,
    ) -> fuse_mt::CallbackResult {
        let fid = match self.get_fid(fh) {
            Err(e) => {
                warn!("read() failed when getting fid: {}", e);
                return callback(Err(libc::EIO));
            }
            Ok(res) => res,
        };

        let mut file = match {
            match self.disk_cache.contains(fid.clone()) {
                Some(path) => File::open(path),
                None => {
                    let url = self.client.get_download_url(fid.clone());
                    let url = match url {
                        Ok(url) => url,
                        Err(e) => {
                            warn!("read() failed when getting URL: {}", e);
                            return callback(Err(libc::EIO));
                        }
                    };
                    if let Err(e) = self.disk_cache.fetch(fid.clone(), url) {
                        warn!("read() failed when downloading: {}", e);
                        return callback(Err(libc::EIO));
                    }
                    let path = match self.disk_cache.contains(fid.clone()) {
                        Some(path) => path,
                        None => {
                            warn!("read() failed when getting path after downloaded");
                            return callback(Err(libc::EIO));
                        }
                    };
                    File::open(path)
                }
            }
        } {
            Ok(file) => file,
            Err(e) => {
                warn!("read() failed when opening cached file: {}", e);
                return callback(Err(libc::EIO));
            }
        };
        if let Err(e) = file.seek(SeekFrom::Start(offset)) {
            warn!("read() failed when seeking cached file: {}", e);
            return callback(Err(libc::EIO));
        }

        let mut data = Vec::<u8>::with_capacity(size as usize);
        if let Err(e) = file.read_to_end(&mut data) {
            warn!("read() failed when reading cached file: {}", e);
            return callback(Err(libc::EIO));
        }

        callback(Ok(&data))
    }

    fn mkdir(
        &self,
        _req: RequestInfo,
        parent: &Path,
        name: &std::ffi::OsStr,
        _mode: u32,
    ) -> ResultEntry {
        let (fid, _parent) = self.req_fid(parent)?;
        self.client
            .mkdir(fid.clone(), name.to_str().ok_or(libc::EINVAL)?.to_string())
            .map_err(|_| libc::EIO)?;
        let list = self.req_update_listing(fid)?;
        let mut found = None;
        let children = list.children.ok_or(libc::ENOTDIR)?;
        for child in children.iter() {
            if child.name == name.to_string_lossy() {
                found = Some(child);
                break;
            }
        }
        let found = match found {
            None => return Err(libc::ENOENT),
            Some(found) => found.clone(),
        };

        Ok((Duration::default(), found.into()))
    }
}

impl RecFs {
    fn req_fid(&self, path: &Path) -> Result<(Fid, Option<Fid>), libc::c_int> {
        let mut parent = None;
        let mut fid = Fid::root();
        let mut is_dir = true;

        for c in path.components().skip(1) {
            // is current fid in cache?
            {
                let map = self.fid_map.read().unwrap();
                if let Some(n) = map.borrow().get_listing(&fid) {
                    let mut found = false;
                    if let Some(children) = &n.children {
                        for child in children.iter() {
                            if child.name == c.as_os_str().to_string_lossy() {
                                info!("found in cache: {:?}", child);
                                parent = Some(fid);
                                fid = child.fid.clone();
                                is_dir = child.ftype == FileType::Directory;
                                found = true;
                                break;
                            }
                        }
                    }

                    if found {
                        continue;
                    }
                }
            }

            // not found in cache, request from server now
            info!("not found in cache: {:?}", c);
            let items = self.client.list(fid.clone()).map_err(|_| libc::ENOENT)?;
            // Update listing
            {
                let mut map = self.fid_map.write().unwrap();
                let mut node = map.borrow_mut().get_listing_mut(fid.clone());
                node.children = Some(items.clone());
                for child in items.iter() {
                    map.borrow_mut()
                        .get_parentmap_mut()
                        .insert(child.fid.clone(), Some(fid.clone()));
                }
            }
            let s = c.as_os_str().to_string_lossy();
            match items.iter().find(|i| i.name == s) {
                Some(item) => {
                    parent = Some(fid);
                    fid = item.fid.clone();
                    is_dir = item.ftype == FileType::Directory;
                }
                None => return Err(libc::ENOENT),
            }
        }
        // if current fid is dir and not in cache (including /), request from server
        let is_in_fidmap = self
            .fid_map
            .read()
            .unwrap()
            .borrow()
            .get_listing(&fid)
            .is_some();
        debug!(
            "fid: {}, is_dir: {}, is_in_fidmap: {}",
            fid, is_dir, is_in_fidmap
        );
        if !is_in_fidmap {
            if is_dir {
                let items = self.client.list(fid.clone()).map_err(|_| libc::ENOENT)?;
                self.fid_map.write().unwrap().borrow_mut().update_fid(
                    &fid,
                    parent.as_ref(),
                    &FidCachedList {
                        children: Some(items),
                    },
                );
            } else {
                self.fid_map.write().unwrap().borrow_mut().update_fid(
                    &fid,
                    parent.as_ref(),
                    &FidCachedList { children: None },
                );
            }
        }
        Ok((fid, parent))
    }

    fn get_fid(&self, fh: u64) -> Result<Fid, libc::c_int> {
        self.fid_map
            .read()
            .unwrap()
            .borrow()
            .get_fid_by_fh(fh)
            .ok_or(libc::EBADF)
    }

    fn get_fid_with_parent(&self, fh: u64) -> Result<(Fid, Option<Fid>), libc::c_int> {
        let map = self.fid_map.read().unwrap();
        let fid = map.borrow().get_fid_by_fh(fh).ok_or(libc::EBADF)?;
        let parent = map.borrow().get_parent_fid(&fid).unwrap();
        Ok((fid, parent))
    }

    fn get_item(&self, fid: Fid, parent: Option<Fid>) -> Result<RecListItem, libc::c_int> {
        let parent = match parent {
            Some(p) => p,
            None => return Ok(RecListItem::root()),
        };
        let map = self.fid_map.read().unwrap();
        let listing = map.get_listing(&parent).ok_or(libc::ENOENT)?;
        listing
            .children
            .as_ref()
            .ok_or(libc::ENOTDIR)?
            .iter()
            .find(|i| i.fid == fid)
            .ok_or(libc::ENOENT)
            .cloned()
    }

    fn get_listing(&self, fid: Fid) -> Result<FidCachedList, libc::c_int> {
        self.fid_map
            .read()
            .unwrap()
            .get_listing(&fid)
            .ok_or(libc::ENOENT)
            .cloned()
    }

    fn req_item(&self, fid: Fid, parent_fid: Option<Fid>) -> Result<RecListItem, libc::c_int> {
        if let Some(parent_fid) = parent_fid {
            let items = self
                .client
                .list(parent_fid.clone())
                .map_err(|_| libc::ENOENT)?;
            // update listing
            self.fid_map
                .write()
                .unwrap()
                .get_listing_mut(parent_fid.clone())
                .children = Some(items.clone());
            Ok(items.into_iter().find(|i| i.fid == fid).unwrap())
        } else {
            Ok(RecListItem::root())
        }
    }

    fn req_update_listing(&self, fid: Fid) -> Result<FidCachedList, libc::c_int> {
        let items = self.client.list(fid.clone()).map_err(|_| libc::ENOENT)?;
        self.fid_map
            .write()
            .unwrap()
            .get_listing_mut(fid.clone())
            .children = Some(items.clone());
        Ok(FidCachedList {
            children: Some(items),
        })
    }
}
