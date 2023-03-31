use crate::fault::Fault;
use crate::models::{AuthEmail, Claims, User};
use crate::util::{self, log, DataRequest, DataResponse, Empty};
use crate::{ACCESS_TOKEN_SECRET, AUTH_EMAIL_COLLECTION, REFRESH_TOKEN_SECRET, USER_COLLECTION};
use chrono::{prelude::*, Duration};
use cosmos_utils::get;
use jsonwebtoken::{encode, EncodingKey, Header}; // decode, Validation, DecodingKey, Algorithm, errors::ErrorKind
use serde::{Deserialize, Serialize};
use warp::reject;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SigninType {
    Email(String),
    OpaqueNid(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<'a> {
    pub access_token: &'a str,
    pub refresh_token: &'a str,
    pub user_id: &'a str,
}

pub async fn signin(
    // Only accept email and password here
    r: DataRequest<String, String>,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let email;
    if let Some(q) = r.data {
        email = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    // Normalise email.
    let email = email.to_lowercase();
    let password;
    if let Some(q) = r.extra {
        password = q;
    } else {
        return Err(reject::custom(Fault::NoExtra));
    }
    let (auth_email, _): (AuthEmail, _) = get(AUTH_EMAIL_COLLECTION, [&email], email.clone())
        .await
        .map_err(|_| reject::custom(Fault::NotFound(format!("Could not find email"))))?;
    if let Some(passhash) = &auth_email.passhash {
        if !util::verify_hash(&passhash, password.as_bytes()) {
            return Err(reject::custom(Fault::WrongPassword));
        }
    } else {
        return Err(reject::custom(Fault::WrongPassword));
    }
    let user_id = auth_email.user_id;

    let (user, _etag): (User, _) = get(USER_COLLECTION, [&user_id], &user_id).await?;

    let iat = Utc::now();
    let exp = iat + Duration::minutes(20);
    let claims = Claims::new(&user_id, exp, &user.roles.clone());

    let access_token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(ACCESS_TOKEN_SECRET.as_ref()),
    ) {
        Ok(token) => token,
        Err(error) => {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not encode access token: {}.",
                error.to_string()
            ))));
        }
    };

    let refresh_token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(REFRESH_TOKEN_SECRET.as_ref()),
    ) {
        Ok(token) => token,
        Err(error) => {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not encode refresh token: {}.",
                error.to_string()
            ))));
        }
    };

    log(format!("User {} successfully logged in", user_id));

    Ok(warp::reply::json(&DataResponse {
        data: Some(&Response {
            access_token: &access_token,
            refresh_token: &refresh_token,
            user_id: &user_id,
        }),
        extra: None::<Empty>,
    }))
}
