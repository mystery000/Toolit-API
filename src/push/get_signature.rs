use crate::push::{Account, Error};
use base64::encode;
use chrono::prelude::*;
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;
use url::form_urlencoded;

pub fn get_signature<'a>(url: &'a str, account: &Account) -> Result<String, Error> {
    let sas_token_expiry = (Utc::now() + chrono::Duration::seconds(86400)).timestamp();
    let sas_token_signature_string = format!("{}\n{}", url, sas_token_expiry);
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match HmacSha256::new_varkey(account.key.as_bytes()) {
        Ok(h) => h,
        Err(err) => {
            return Err(Error::Unspecified(format!(
                "Error generating signature: {}.",
                err.to_string()
            )));
        }
    };
    mac.update(&sas_token_signature_string.as_bytes());
    let escaped_sas_token_signature: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("sig", &encode(mac.finalize().into_bytes()))
        .finish();

    let signature = format!(
        "SharedAccessSignature se={}&{}&skn={}&sr={}",
        sas_token_expiry, escaped_sas_token_signature, account.key_name, url
    );

    return Ok(String::from(signature.as_str()));
}
