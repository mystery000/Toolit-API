use crate::models::User;
use crate::push::{self, Account, Error, Pns, DIRECT_PUSH_ENCODED_URL};
use crate::util::log;
use std::time::Duration;

pub async fn register_device_success<'a>(
    user: &'a User,
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

    for d in &user.devices {
        let pns = match d.os.as_str() {
            "iOS" => Pns::Apple,
            "Android" => Pns::Gcm,
            _ => {
                log(format!(
                    "Could not register_device_success PN due to unrecognized device os"
                ));
                continue;
            }
        };

        let body = match pns {
            Pns::Apple => format!(
                "{{
                    \"aps\" : {{ 
                        \"alert\": \"Nu får du pushnotiser.\",
                        \"sound\": \"default\"
                    }}
                }}"
            ),
            Pns::Gcm => format!(
                "{{
                    \"notification\": {{ 
                        \"body\": \"Nu får du pushnotiser.\",
                        \"sound\": \"default\"
                    }}
                }}"
            ),
        };

        match push::send(&d.handle, &pns, &body, &signature, timeout).await {
            Ok(()) => {}
            Err(e) => {
                log(format!("Could not register_device_success PN due to {}", e));
            }
        }
    }

    Ok(())
}
