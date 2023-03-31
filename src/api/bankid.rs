use crate::fault::Fault;
use crate::models::{AuthEmail, AuthNid, Claims, User};
use crate::util::{encrypt_string, DataRequest, DataResponse, Empty, SecretKey};
use crate::{
    ACCESS_TOKEN_SECRET, AUTH_EMAIL_COLLECTION, AUTH_NID_COLLECTION, BANKID_CERT_PATH,
    BANKID_IDENT_PASS, BANKID_IDENT_PATH, BANKID_NID_SECRET, REFRESH_TOKEN_SECRET, USER_COLLECTION,
};
use bankid::{AuthRequest, BankIdClient, Status};
use chrono::{prelude::*, Duration};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use std::net::SocketAddr;
use warp::reject; // decode, Validation, DecodingKey, Algorithm, errors::ErrorKind

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
#[serde(untagged)]
pub enum BankIdResponse {
    #[serde(rename_all = "camelCase")]
    BankIdTokenResponse {
        order_ref: String,
        auto_start_token: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    BankIdResponse {
        opaque_nid: String,
        first_name: String,
        last_name: String,
    },
    #[serde(rename_all = "camelCase")]
    SignInResponse {
        access_token: String,
        refresh_token: String,
        user_id: String,
    },
    #[serde(rename_all = "camelCase")]
    // We return the number of ticks. 1 tick = 100 nanoseconds
    BankIdRetryResponse { retry_in: u128 },
}

// Bankid endpoint should initialize bankid process if no order-ref is provided in the data.
// It should take a personal number as the optional extra.
pub async fn bankid(
    r: DataRequest<String, String>,
    socket_addr: Option<std::net::SocketAddr>,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let order_ref = r.data;
    let personal_number = r.extra;
    let socket_addr = match socket_addr {
        Some(s) => s,
        None => {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not get the ip address required to log in with bankid"
            ))));
        }
    };
    match order_ref {
        None => bankid_init(socket_addr, personal_number).await,
        Some(order_ref) => bankid_signin(order_ref).await,
    }
}

async fn bankid_signin(order_ref: String) -> Result<warp::reply::Json, warp::Rejection> {
    let client = BankIdClient::new(&*BANKID_IDENT_PATH, &BANKID_IDENT_PASS, &*BANKID_CERT_PATH)
        .await
        .map_err(|e| reject::custom(Fault::from(e)))?;

    let collect = client
        .collect(order_ref.clone())
        .await
        .map_err(|e| reject::custom(Fault::from(e)))?;

    let (nid, first_name, last_name) = match collect.status {
        Status::Pending => {
            return Ok(warp::reply::json(&DataResponse {
                //Magic number is 2 seconds in ticks.
                data: Some(&BankIdResponse::BankIdRetryResponse {
                    // We return the number of ticks. 1 tick = 100 nanoseconds
                    retry_in: std::time::Duration::from_secs(2).as_nanos() / 100,
                }),
                extra: None::<Empty>,
            }));
        }
        Status::Failed => {
            return Err(reject::custom(Fault::Unspecified(String::from(
                "Bank ID poll failed",
            ))))
        }
        Status::Complete(data) => (
            data.user.personal_number,
            data.user.given_name,
            data.user.surname,
        ),
    };

    let user = match cosmos_utils::get(AUTH_NID_COLLECTION, [&nid], &nid).await {
        Ok(r) => {
            let (auth_nid, _): (AuthNid, _) = r;
            let user_id = auth_nid.user_id;
            let (user, _etag): (User, _) =
                cosmos_utils::get(USER_COLLECTION, [&user_id], &user_id).await?;
            user
        }
        Err(e) => match e.kind {
            // If we could not find the nid then we return an opaque nid that can be used to signup
            cosmos_utils::CosmosErrorKind::NotFound => {
                let code = match SecretKey::from_slice(BANKID_NID_SECRET.as_bytes()) {
                    Ok(r) => r,
                    Err(_) => {
                        return Err(reject::custom(Fault::Unspecified(String::from(
                            "Internal server error, could not generate nid secret",
                        ))))
                    }
                };
                let opaque_nid = encrypt_string(nid, &code)?;

                return Ok(warp::reply::json(&DataResponse {
                    data: Some(&BankIdResponse::BankIdResponse {
                        first_name,
                        last_name,
                        opaque_nid,
                    }),
                    extra: None::<Empty>,
                }));
            }
            // Return error if signin problem was not due to nid not being found
            _ => {
                return Err(reject::custom(Fault::Unspecified(String::from(
                    "Could not sign in due to database error",
                ))));
            }
        },
    };

    // NOTE: If we login with bankid we need to make sure that we delete the password of the user so
    // they can never login without bankid in the future
    cosmos_utils::modify(
        AUTH_EMAIL_COLLECTION,
        [&user.email],
        &user.email,
        |mut auth_email: AuthEmail| {
            auth_email.passhash = None;
            Ok(auth_email)
        },
    )
    .await
    .map_err(|_| {
        reject::custom(Fault::Unspecified(format!(
            "Could not remove password from user"
        )))
    })?;

    let iat = Utc::now();
    // FIXME(Jonathan): Temporary, make 20 minutes when bankid problem fixed
    let exp = iat + Duration::days(90);
    let claims = Claims::new(&user.id, exp, &user.roles.clone());

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

    Ok(warp::reply::json(&DataResponse {
        data: Some(&BankIdResponse::SignInResponse {
            access_token,
            refresh_token,
            user_id: user.id,
        }),
        extra: None::<Empty>,
    }))
}

async fn bankid_init(
    socket_addr: SocketAddr,
    personal_number: Option<String>,
) -> Result<warp::reply::Json, warp::Rejection> {
    let client = BankIdClient::new(&*BANKID_IDENT_PATH, &BANKID_IDENT_PASS, &*BANKID_CERT_PATH)
        .await
        .map_err(|e| reject::custom(Fault::from(e)))?;

    let auth_req = AuthRequest {
        personal_number,
        end_user_ip: socket_addr.ip(),
        requirement: None,
    };

    let auth_resp = client
        .auth(auth_req)
        .await
        .map_err(|e| reject::custom(Fault::from(e)))?;
    let order_ref = auth_resp.order_ref;
    let auto_start_token = auth_resp.auto_start_token;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&BankIdResponse::BankIdTokenResponse {
            order_ref,
            auto_start_token: Some(auto_start_token),
        }),
        extra: None::<Empty>,
    }))
}
