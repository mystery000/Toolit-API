use crate::models::User;
use crate::push::{self, Account, Error, Pns, DIRECT_PUSH_ENCODED_URL};
use crate::util::log;
use futures::future::join_all;
use std::time::Duration;

pub async fn send_custom_pn<'a>(
    user: &'a User,
    message: &str,
    timeout: Option<Duration>,
    account: &Account,
) -> Result<(), Error> {
    let signature = match push::get_signature(DIRECT_PUSH_ENCODED_URL, account) {
        Ok(g) => g,
        Err(err) => {
            return Err(Error::Unspecified(format!(
                "Error generating signature: {}.",
                err.to_string()
            )));
        }
    };

    let timeout = match timeout {
        Some(t) => t,
        None => Duration::new(15, 0),
    };

    let user_devices = &user.devices;
    {
        let devices: Vec<_> = user_devices
            .iter()
            .map(|d| {
                let pns = match d.os.as_str() {
                    "iOS" => Pns::Apple,
                    "Android" => Pns::Gcm,
                    _ => {
                        log(format!(
                            "Could not send custom PN due to unrecognized device os"
                        ));
                        return None;
                    }
                };
                let body = match pns {
                    Pns::Apple => format!(
                        "{{
                    \"aps\" : {{ 
                        \"alert\": \"{}\",
                        \"sound\": \"default\"
                    }}
                }}",
                        message
                    ),
                    Pns::Gcm => format!(
                        "{{
                    \"notification\": {{ 
                        \"body\": \"{}\",
                        \"sound\": \"default\"
                    }}
                }}",
                        message
                    ),
                };
                Some((&d.handle, pns, body))
            })
            .filter(|o| o.is_some())
            .map(|o| o.unwrap())
            .collect();
        let mut futs = vec![];
        for (handle, pns, body) in &devices {
            futs.push(push::send(handle, &pns, &body, &signature, timeout));
        }

        let futs_r = join_all(futs).await;
        for f in &futs_r {
            match f {
                Ok(()) => (),
                Err(e) => {
                    return Err(Error::Unspecified(format!(
                        "Could not send custom PN due to {}",
                        e
                    )));
                }
            }
        }
    }

    Ok(())
}
