use crate::fault::Fault;
use crate::models::{AuthEmail, Claims, RoleFlags, User};
use crate::util::{self, DataRequest, DataResponse, Empty};
use crate::{AUTH_EMAIL_COLLECTION, USER_COLLECTION};
use cosmos_utils::get;
use cosmos_utils::upsert;
use warp::reject;

pub async fn change_password(
    user_id: String,
    r: DataRequest<String, String>,
    claims: Option<Claims>,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_password;
    if let Some(q) = r.data {
        new_password = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }
    let old_password = r.extra;
    let (user, _etag): (User, _) = get(USER_COLLECTION, [&user_id], &user_id).await?;
    // Normalise email.
    let email = user.email.to_lowercase();

    let (mut auth_email, etag): (AuthEmail, _) =
        get(AUTH_EMAIL_COLLECTION, [&email], &email).await?;

    if let Some(old_password) = old_password {
        if let Some(passhash) = &auth_email.passhash {
            // Check old password.
            if !util::verify_hash(&passhash, old_password.as_bytes()) {
                return Err(reject::custom(Fault::WrongPassword));
            }
        } else {
            return Err(reject::custom(Fault::WrongPassword));
        }
    } else if let Some(claims) = claims {
        // If the calling user is an office personnel admin for some office they can change the
        // password
        let mut ok = false;
        for e in &claims.rol {
            if let Some(_) = &e.sub {
                if e.flg.contains(RoleFlags::OFFICE_PERSONNEL_ADMIN) {
                    ok = true;
                    break;
                }
            }
        }
        if !ok {
            return Err(reject::custom(Fault::NoExtra));
        }
    } else {
        return Err(reject::custom(Fault::NoExtra));
    }

    // Set new password.
    auth_email.passhash = Some(util::hash(new_password.as_bytes())); // Calculate pass hash.

    upsert(
        AUTH_EMAIL_COLLECTION,
        [&auth_email.id],
        &auth_email,
        Some(&etag),
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: None::<Empty>,
        extra: None::<Empty>,
    }))
}
