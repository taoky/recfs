use std::convert::TryInto;

use libaes::Cipher;
use serde::Deserialize;
use serde_json::json;
use log::info;

use crate::status_check;

use super::{RecClient, AESKEY, CLIENTID, SIGNATURE};

#[derive(Debug)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Default)]
pub struct RecAuth {
    pub token: Option<Token>,
}

#[derive(Deserialize)]
pub struct RecTempTicketEntity {
    tempticket: String,
}

#[derive(Deserialize)]
struct RecEncryptedEntity {
    msg_encrypt: String,
}

#[derive(Deserialize, Debug)]
struct RecUserAuthResponse {
    gid: String,
    username: String,
    name: String,
    x_auth_token: String,
    refresh_token: String,
}

impl RecAuth {
    pub fn get_tempticket(client: &RecClient) -> anyhow::Result<String> {
        let body = client.get_noretry::<_, RecTempTicketEntity>(
            "client/tempticket",
            false,
            &[("clientid", CLIENTID)],
        )?;
        status_check!(body);
        Ok(body.entity.unwrap().tempticket)
    }

    fn aes_encrypt(data: &str) -> anyhow::Result<String> {
        let cipher = Cipher::new_128(AESKEY);
        let mut iv = AESKEY.clone();
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
        let mut iv = AESKEY.clone();
        iv.reverse();

        let encrypted = base64::decode(data)?;
        let decrypted = cipher.cbc_decrypt(&iv, &encrypted);
        if strip {
            let data_len = u32::from_be_bytes(decrypted[0..4].try_into()?);
            let data = String::from_utf8(decrypted[4..(4 + data_len as usize)].to_vec())?;
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
        let tempticket = RecAuth::get_tempticket(&client)?;

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
            .to_string()
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
        info!("{}", sign);
        let md5sign = format!("{:X}", md5::compute(sign));

        let response = client.post_noretry::<_, RecEncryptedEntity>(
            format!("user/login?tempticket={}&sign={}", tempticket, md5sign).as_str(),
            false,
            &[("msg_encrypt", encrypted_string)],
        )?;
        status_check!(response);
        let decrypted_string = RecAuth::aes_decrypt(&response.entity.unwrap().msg_encrypt, true)?;
        let userauth = serde_json::from_str::<RecUserAuthResponse>(&decrypted_string)?;
        info!("{:?}", userauth);

        unimplemented!()
    }
}
