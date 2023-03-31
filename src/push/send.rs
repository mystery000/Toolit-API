use crate::push::{Error, Pns, DIRECT_PUSH_URL};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::ClientBuilder;
use std::time::Duration;

pub async fn send<'a>(
    handle: &'a str,
    pns: &Pns,
    body: &'a str,
    signature: &'a str,
    timeout: Duration,
) -> Result<(), Error> {
    let auth_header = match HeaderValue::from_str(&signature) {
        Ok(h) => h,
        Err(err) => {
            return Err(Error::Unspecified(format!(
                "Error generating auth header: {}.",
                err.to_string()
            )));
        }
    };

    let handle = match HeaderValue::from_str(handle) {
        Ok(h) => h,
        Err(err) => {
            return Err(Error::Unspecified(format!(
                "Error generating handle header: {}.",
                err.to_string()
            )));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, auth_header);
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json;charset=utf-8"),
    );
    headers.insert("ServiceBusNotification-DeviceHandle", handle);

    let format = match pns {
        Pns::Apple => HeaderValue::from_static("apple"),
        Pns::Gcm => HeaderValue::from_static("gcm"),
    };
    headers.insert("ServiceBusNotification-Format", format);

    headers.insert("x-ms-version", HeaderValue::from_static("2015-0"));

    let client = match ClientBuilder::new().timeout(timeout).build() {
        Ok(c) => c,
        Err(err) => {
            return Err(Error::Unspecified(format!(
                "Error building request: {}.",
                err.to_string()
            )));
        }
    };
    let res = match client
        .post(format!("{}", DIRECT_PUSH_URL).as_str())
        .headers(headers)
        .body(String::from(body))
        .send()
        .await
    {
        Ok(r) => r,
        Err(err) => {
            return Err(Error::Unspecified(format!("Error: {}.", err.to_string())));
        }
    };

    let status = res.status();
    if status.is_success() {
        Ok(())
    } else {
        let g = match res.text().await {
            Ok(x) => x,
            Err(_) => String::from("<no body>"),
        };

        Err(Error::Unspecified(format!("Error : {} / {}.", status, g)))
    }
}
