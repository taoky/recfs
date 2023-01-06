pub mod auth;
pub mod list;
pub mod stat;

use std::sync::{Arc, Mutex};

use binary_macros::base64;
use fuse_mt::FileType;
use log::warn;
use reqwest::blocking::Client;
use serde::Deserializer;
use serde::{Deserialize, Serialize};

use self::auth::RecAuth;

const APIURL: &str = "https://recapi.ustc.edu.cn/api/v2/";
const CLIENTID: &str = "d5485a8c-fecb-11e9-b690-005056b70c02";
const SIGNATURE: &str = "VZPDF6HxKyh0hhqFqY2Tk6udzlambRgK";
static AESKEY: &[u8; 16] = base64!("Z1pNbFZmMmVqd2wwVmlHNA==");

type EmptyQuery = [(String, String); 0];

#[macro_export]
macro_rules! status_check {
    ($x: expr) => {
        if $x.status_code != 200 {
            return Err(anyhow::anyhow!(
                "Status code {}, error message: {}",
                $x.status_code,
                $x.message
            ));
        }
    };
}

pub struct RecClient {
    pub auth: Arc<Mutex<RecAuth>>,
    client: Client,
}

#[derive(Deserialize, Debug)]
pub struct RecRes<T>
where
    T: Default + for<'a> Deserialize<'a>,
{
    // entity exists when succeed
    #[serde(deserialize_with = "failure_to_default")]
    entity: T,
    // message exists when failed
    message: String,
    status_code: i32,
}

fn failure_to_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let key = match Option::<T>::deserialize(de) {
        Ok(key) => key,
        Err(e) => {
            warn!("Serde gets undeserialize data: {}", e);
            return Ok(T::default());
        }
    };
    Ok(key.unwrap_or_default())
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

    pub fn get_noretry<T: Serialize + ?Sized, S: for<'a> Deserialize<'a> + Default>(
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

    pub fn get<T: Serialize + ?Sized, S: for<'a> Deserialize<'a> + Default>(
        &self,
        path: &str,
        query: &T,
    ) -> anyhow::Result<RecRes<S>> {
        let res = self.get_noretry(path, true, query)?;
        if res.status_code == 401 {
            let auth = self.auth.clone();
            let mut auth = auth.lock().unwrap();
            auth.refresh(self)?;
            Ok(self.get_noretry(path, true, query)?)
        } else {
            Ok(res)
        }
    }

    pub fn post_noretry<T: Serialize + ?Sized, S: for<'a> Deserialize<'a> + Default>(
        &self,
        path: &str,
        token: bool,
        json: &T,
        headers: Option<&[(String, String)]>,
    ) -> anyhow::Result<RecRes<S>> {
        assert!(!(token && headers.is_some()));
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
        if let Some(headers) = headers {
            for (key, value) in headers {
                builder = builder.header(key, value);
            }
        }
        let res = builder.json(json).send()?;

        let body = serde_json::from_str::<RecRes<S>>(res.text()?.trim_start_matches('\u{feff}'))?;

        Ok(body)
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
