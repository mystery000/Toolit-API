use crate::fault::Fault;
use crate::models::{Claims, User};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::{ACCESS_TOKEN_SECRET, REFRESH_TOKEN_SECRET, USER_COLLECTION};
use chrono::{prelude::*, Duration};
use cosmos_utils::get;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use warp::reject;

pub async fn refresh_token(
    user_id: String,
    r: DataRequest<String, Empty>,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let req;
    if let Some(q) = r.data {
        req = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    // Validate refresh token.
    let validation = Validation {
        validate_exp: false,
        ..Validation::default()
    };
    match decode::<Claims>(
        &req,
        &DecodingKey::from_secret(REFRESH_TOKEN_SECRET.as_ref()),
        &validation,
    ) {
        Ok(c) => {
            if user_id != c.claims.sub {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "User id in url does not match token ({} != {}).",
                    user_id, c.claims.sub
                ))));
            } else {
                // TODO: Check blacklist.
            }
        }
        Err(error) => {
            return Err(reject::custom(Fault::IllegalArgument(format!(
                "Could not decode token: {}.",
                error.to_string()
            ))))
        }
    };

    let (user, _etag): (User, _) = get(USER_COLLECTION, [&user_id], user_id.clone()).await?;
    if user.deleted {
        return Err(reject::custom(Fault::Forbidden(
            format!("User is deleted",),
        )));
    }
    let iat = Utc::now();
    let exp = iat + Duration::minutes(20);
    let claims = Claims::new(&user_id, exp, &user.roles);

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

    Ok(warp::reply::json(&DataResponse {
        data: Some(access_token),
        extra: None::<Empty>,
    }))
}
