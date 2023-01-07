use std::{
    collections::HashMap,
    io::Write,
    path::{PathBuf, Path},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use file_lock::FileOptions;
use log::{info, warn};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::fid::Fid;

pub struct Cache {
    basepath: PathBuf,
    create_counter: AtomicUsize,
    create_mapping: Arc<Mutex<HashMap<Fid, (Fid, String)>>>,
}

impl Default for Cache {
    fn default() -> Self {
        let basepath = std::env::temp_dir().join("recfs").join(
            thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(char::from)
                .collect::<String>(),
        );
        Cache::init_path(&basepath);
        Self {
            basepath,
            create_counter: AtomicUsize::new(0),
            create_mapping: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Cache {
    fn init_path(path: &PathBuf) {
        info!("Cache folder: {}", path.display());
        if !path.exists() {
            std::fs::create_dir_all(path).unwrap();
        } else {
            let mut input = String::new();
            print!(
                "Remove folder {} for initialization? [y/N] ",
                path.display()
            );
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim() == "y" {
                std::fs::remove_dir_all(path).unwrap();
            } else {
                warn!("Ignore initialization of cache folder {}.", path.display());
            }
        }
    }

    // avoid a URL request
    pub fn contains(&self, fid: Fid) -> Option<String> {
        let path = self.basepath.join(fid.to_string());
        if path.exists() {
            path.to_str().map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn fetch(&self, fid: Fid, url: String) -> anyhow::Result<()> {
        let download_path = self
            .basepath
            .join(format!("{}.{}", fid.to_string(), "download"));
        let final_path = self.basepath.join(fid.to_string());
        let mut download_lock = file_lock::FileLock::lock(
            download_path.clone(),
            true,
            FileOptions::new().write(true).create(true),
        )?;
        if final_path.exists() {
            return Ok(());
        }
        let mut resp = reqwest::blocking::get(&url)?;
        std::io::copy(&mut resp, &mut download_lock.file)?;
        // rename
        std::fs::rename(download_path, final_path)?;
        Ok(())
    }

    pub fn create(&self, parent: Fid, name: String) -> anyhow::Result<Fid> {
        info!("Cache: Try creating file {} under {}", name, parent);
        let id = self.create_counter.fetch_add(1, Ordering::SeqCst);
        let fid_name = format!("{}{}", "write-", id);
        let path = self.basepath.join(fid_name.clone());
        std::fs::File::create(path)?;
        let fid = Fid::from(fid_name);
        self.create_mapping
            .lock()
            .unwrap()
            .insert(fid.clone(), (parent, name));
        Ok(fid)
    }

    pub fn pop_created_info(&self, fid: Fid) -> Option<(Fid, String)> {
        self.create_mapping.lock().unwrap().remove(&fid)
    }

    pub fn get_created_path(&self, fid: Fid) -> PathBuf {
        assert!(fid.is_created());
        self.basepath.join(fid.to_string())
    }
}
