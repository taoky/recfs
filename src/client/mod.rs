pub mod auth;
pub mod list;

use std::sync::{Arc, Mutex};

use binary_macros::base64;
use fuse_mt::FileType;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use self::auth::RecAuth;

const APIURL: &str = "https://recapi.ustc.edu.cn/api/v2/";
const CLIENTID: &str = "d5485a8c-fecb-11e9-b690-005056b70c02";
const SIGNATURE: &str = "VZPDF6HxKyh0hhqFqY2Tk6udzlambRgK";
static AESKEY: &[u8; 16] = base64!("Z1pNbFZmMmVqd2wwVmlHNA==");

#[macro_export]
macro_rules! status_check {
    ($x: expr) => {
        if $x.status_code != 200 {
            return Err(anyhow::anyhow!(
                "Status code {}, error message: {}",
                $x.status_code,
                $x.message.unwrap_or("".to_owned())
            ));
        }
    };
}

pub struct RecClient {
    pub auth: Arc<Mutex<RecAuth>>,
    client: Client,
}

#[derive(Deserialize)]
pub struct RecRes<T> {
    // entity exists when succeed
    entity: Option<T>,
    // message exists when failed
    message: Option<String>,
    status_code: i32,
}

impl Default for RecClient {
    fn default() -> Self {
        Self {
            auth: Arc::new(Mutex::new(RecAuth::default())),
            client: Client::new(),
        }
    }
}

impl RecClient {
    pub fn set_auth(&mut self, auth: RecAuth) {
        *self.auth.lock().unwrap() = auth;
    }

    pub fn get_noretry<T: Serialize + ?Sized, S: for<'a> Deserialize<'a>>(
        &self,
        path: &str,
        token: bool,
        query: &T,
    ) -> anyhow::Result<RecRes<S>> {
        let url = format!("{}{}", APIURL, path);
        let mut builder = self.client.get(url);
        if token {
            let auth = self.auth.clone();
            let auth = auth.lock().unwrap();
            builder = builder.header(
                "x-auth-token",
                auth.token.as_ref().unwrap().access_token.as_str(),
            );
        }
        let res = builder.query(query).send()?;
        let body = serde_json::from_str::<RecRes<S>>(res.text()?.trim_start_matches('\u{feff}'))?;

        Ok(body)
    }

    pub fn get<T: Serialize + ?Sized, S: for<'a> Deserialize<'a>>(
        &self,
        path: &str,
        query: &T,
    ) -> anyhow::Result<RecRes<S>> {
        let res = self.get_noretry(path, true, query)?;
        if res.status_code == 401 {
            // TODO: refresh token
            Ok(self.get_noretry(path, true, query)?)
        } else {
            Ok(res)
        }
    }

    pub fn post_noretry<T: Serialize + ?Sized, S: for<'a> Deserialize<'a>>(
        &self,
        path: &str,
        token: bool,
        json: &T,
    ) -> anyhow::Result<RecRes<S>> {
        let url = format!("{}{}", APIURL, path);
        let mut builder = self.client.post(url);
        if token {
            let auth = self.auth.clone();
            let auth = auth.lock().unwrap();
            builder = builder.header(
                "x-auth-token",
                auth.token.as_ref().unwrap().access_token.as_str(),
            );
        }
        let res = builder.json(json).send()?;
        println!("{:?}", res.text());
        unimplemented!();
        // let body = serde_json::from_str::<RecRes<S>>(res.text()?.trim_start_matches('\u{feff}'))?;

        // Ok(body)
    }
}

pub fn filename(name: String, ext: String) -> String {
    if ext.is_empty() {
        name
    } else {
        format!("{}.{}", name, ext)
    }
}

pub fn filetype(ftype: &str) -> anyhow::Result<FileType> {
    match ftype {
        "folder" => Ok(FileType::Directory),
        "file" => Ok(FileType::RegularFile),
        _ => Err(anyhow::Error::msg("Unknown file type ".to_owned() + ftype)),
    }
}
