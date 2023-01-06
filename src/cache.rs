use std::{io::Write, path::PathBuf};

use file_lock::FileOptions;
use log::warn;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::fid::Fid;

pub struct Cache {
    basepath: PathBuf,
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
        Self { basepath }
    }
}

impl Cache {
    fn init_path(path: &PathBuf) {
        println!("Cache folder: {}", path.display());
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
}
