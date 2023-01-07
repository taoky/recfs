use std::{collections::HashMap, convert::TryInto, io::Write};

use libaes::Cipher;
use log::info;
use rpassword::read_password;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::status_check;

use super::{RecClient, AESKEY, CLIENTID, SIGNATURE};

#[derive(Debug, Deserialize, Serialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Default)]
pub struct RecAuth {
    pub token: Option<Token>,
}

#[derive(Deserialize, Default)]
pub struct RecTempTicketEntity {
    tempticket: String,
}

#[derive(Deserialize, Default)]
struct RecEncryptedEntity {
    msg_encrypt: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct RecUserAuthResponse {
    gid: String,
    username: String,
    name: String,
    x_auth_token: String,
    refresh_token: String,
}

#[derive(Deserialize, Debug)]
struct RecUserAuthRefreshResponse {
    x_auth_token: String,
    refresh_token: String,
}

static SERVICENAME: &str = "recfs";

#[derive(Debug)]
pub enum RecAuthMethod {
    UsernamePassword(String, String),
    Cookie(String, String),
}

impl RecAuth {
    pub fn get_tempticket(client: &RecClient) -> anyhow::Result<String> {
        let body = client.get_noretry::<_, RecTempTicketEntity>(
            "client/tempticket",
            false,
            &[("clientid", CLIENTID)],
        )?;
        status_check!(body);
        Ok(body.entity.tempticket)
    }

    fn aes_encrypt(data: &str) -> anyhow::Result<String> {
        let cipher = Cipher::new_128(AESKEY);
        let mut iv = *AESKEY;
        iv.reverse();

        let data_len: u32 = data.len().try_into()?;
        let mut payload = Vec::new();
        payload.extend_from_slice(&data_len.to_be_bytes());
        payload.extend_from_slice(data.as_bytes());
        let encrypted = cipher.cbc_encrypt(&iv, &payload);
        Ok(base64::encode(encrypted))
    }

    fn aes_decrypt(data: &str, strip: bool) -> anyhow::Result<String> {
        let cipher = Cipher::new_128(AESKEY);
        let mut iv = *AESKEY;
        iv.reverse();

        let encrypted = base64::decode(data)?;
        let decrypted = cipher.cbc_decrypt(&iv, &encrypted);
        if strip {
            info!("{:?}", std::str::from_utf8(&decrypted));
            let data = String::from_utf8(decrypted[16..].to_vec())?;
            Ok(data)
        } else {
            Ok(String::from_utf8(decrypted)?)
        }
    }

    fn serialize_dict(dict: &[(String, String)]) -> String {
        let sorted_dict = {
            let mut dict = dict.to_vec();
            dict.sort_by(|a, b| a.0.cmp(&b.0));
            dict
        };
        let list = sorted_dict
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>();
        list.join("&")
    }

    pub fn login(
        &mut self,
        client: &RecClient,
        cas_username: String,
        cas_password: String,
    ) -> anyhow::Result<()> {
        let tempticket = RecAuth::get_tempticket(client)?;

        let string = format!(
            "{}{}",
            "A".repeat(12),
            json!({
                "username": cas_username,
                "password": cas_password,
                "device_type": "PC",
                "client_terminal_type": "client",
                "type": "nusoap"
            })
        );
        let encrypted_string = RecAuth::aes_encrypt(&string)?;
        let sign = format!(
            "{}{}",
            SIGNATURE,
            RecAuth::serialize_dict(&[
                ("tempticket".to_string(), tempticket.clone()),
                ("msg_encrypt".to_string(), encrypted_string.clone())
            ])
        );
        let md5sign = format!("{:X}", md5::compute(sign));

        let response = client.post_noretry::<_, RecEncryptedEntity>(
            format!("user/login?tempticket={}&sign={}", tempticket, md5sign).as_str(),
            false,
            &json!({ "msg_encrypt": encrypted_string }),
            None,
        )?;
        status_check!(response);
        let decrypted_string = RecAuth::aes_decrypt(&response.entity.msg_encrypt, true)?;
        let userauth = serde_json::from_str::<RecUserAuthResponse>(&decrypted_string)?;
        info!("{:?}", userauth);

        self.token = Some(Token {
            access_token: userauth.x_auth_token,
            refresh_token: userauth.refresh_token,
        });
        self.set_keyring()?;
        Ok(())
    }

    fn parse_cookie(cookie: &str) -> (String, String) {
        let cookies: HashMap<_, _> = cookie
            .trim()
            .trim_matches('"')
            .split("; ")
            .map(|x| {
                let v = x.split_once('=').expect("Unexpected cookie format");
                (v.0, v.1)
            })
            .collect();
        let auth_token = cookies
            .get("Rec-Token")
            .expect("No Rec-Token found in cookie");
        let refresh_token = cookies
            .get("Rec-RefreshToken")
            .expect("No Rec-RefreshToken found in cookie");
        let refresh_token: serde_json::Value = serde_json::from_str(
            &urlencoding::decode(refresh_token).expect("Cannot unescape refresh token"),
        )
        .expect("Cannot parse refresh token");
        let refresh_token = refresh_token["refresh_token"]
            .as_str()
            .expect("Cannot get refresh token from parsed data");
        (auth_token.to_owned().to_string(), refresh_token.to_owned())
    }

    pub fn interactive() -> RecAuthMethod {
        println!(
            "By default, your CAS username and password will be sent to https://recapi.ustc.edu.cn"
        );
        println!("You can login in browser manually and paste the output of `document.cookie` in Developer Console instead");
        let mut username = String::new();
        print!("Username or cookie: ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut username).unwrap();
        if username.contains("Rec-Token") {
            let (auth_token, refresh_token) = RecAuth::parse_cookie(&username);
            RecAuthMethod::Cookie(auth_token, refresh_token)
        } else {
            print!("Password: ");
            std::io::stdout().flush().unwrap();
            let password = read_password().unwrap();
            RecAuthMethod::UsernamePassword(username.trim().to_string(), password)
        }
    }

    pub fn try_keyring(&mut self) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICENAME, "userauth");
        let userauth_json = entry.get_password()?;
        let userauth = serde_json::from_str::<Token>(&userauth_json)?;
        self.token = Some(userauth);
        Ok(())
    }

    fn set_keyring(&mut self) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICENAME, "userauth");
        let userauth_json = serde_json::to_string(&self.token.as_ref().unwrap())?;
        entry.set_password(&userauth_json)?;
        Ok(())
    }

    pub fn clear_keyring(&self) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICENAME, "userauth");
        entry.delete_password()?;
        Ok(())
    }

    pub fn refresh(&mut self, client: &RecClient) -> anyhow::Result<()> {
        let resp = client.post_noretry::<_, RecEncryptedEntity>(
            "user/refresh/token",
            false,
            &json!({
                "clientid": CLIENTID,
                "refresh_token": self.token.as_ref().unwrap().refresh_token
            }),
            Some(&[(
                "X-auth-token".to_owned(),
                self.token.as_ref().unwrap().access_token.to_owned(),
            )]),
        )?;
        status_check!(resp);
        let decrypted_string = RecAuth::aes_decrypt(&resp.entity.msg_encrypt, false)?;
        info!("{}", decrypted_string);
        let refresh_auth = serde_json::from_str::<RecUserAuthRefreshResponse>(&decrypted_string)?;
        self.token = Some(Token {
            access_token: refresh_auth.x_auth_token,
            refresh_token: refresh_auth.refresh_token,
        });
        self.set_keyring()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_parse() {
        let cookie = r#""Rec-Storage=moss; Rec-Token=aaaaaaabbaaa3aaaaaaaaaaaaaaaaaa1; Rec-RefreshToken={%22refresh_token%22:%22zzzzzzzz22zzzzzzzzzazzzz9zzzzzzz%22%2C%22token_expire_time%22:%222077-11-04%2005:14:19%22}""#;
        let (auth_token, refresh_token) = RecAuth::parse_cookie(cookie);
        assert_eq!(auth_token, "aaaaaaabbaaa3aaaaaaaaaaaaaaaaaa1");
        assert_eq!(refresh_token, "zzzzzzzz22zzzzzzzzzazzzz9zzzzzzz");
    }
}
