use crate::fault::Fault;
use crate::models::{Claims, RoleFlags, User};
use crate::util::SecretKey;
use crate::util::{encrypt_optional_string, encrypt_string, has_role, log, DataResponse, Empty};
use crate::{AUTH_EMAIL_COLLECTION, AUTH_NID_COLLECTION, USER_COLLECTION};
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn user_delete(
    user_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    if claims.sub != user_id
        && !has_role(
            None,
            &claims,
            RoleFlags::OFFICE_PERSONNEL_ADMIN & RoleFlags::GLOBAL_PERSONNEL_ADMIN,
        )
    {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Need to be the user or an admin to delete users"
        ))));
    }

    // Get email for later use.
    let (user, _etag): (User, _) = cosmos_utils::get(USER_COLLECTION, [&user_id], &user_id).await?;
    let email = user.email.to_lowercase(); // Normalize.
    let nid = user.nid;

    let code = SecretKey::default();
    let deleted_user = modify(USER_COLLECTION, [&user_id], &user_id, |mut user: User| {
        if user.id != user_id {
            return Err(reject::custom(Fault::IllegalArgument(format!(
                "user_id does not match url ({} != {}).",
                user.id, user_id
            ))));
        }

        user.nid = encrypt_string(user.nid, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt nid in delete_user due to {:?}",
                e
            ));
            String::new()
        });
        user.first_name = encrypt_string(user.first_name, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt first name in delete_user due to {:?}",
                e
            ));
            String::new()
        });
        user.preferred_name =
            encrypt_optional_string(user.preferred_name, &code).unwrap_or_else(|e| {
                log(format!(
                    "Could not encrypt preferred name in delete_user due to {:?}",
                    e
                ));
                None
            });
        user.middle_names = encrypt_optional_string(user.middle_names, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt middle names in delete_user due to {:?}",
                e
            ));
            None
        });
        user.last_name = encrypt_string(user.last_name, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt last name in delete_user due to {:?}",
                e
            ));
            String::new()
        });
        user.images = vec![];
        user.email = encrypt_string(user.email, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt email in delete_user due to {:?}",
                e
            ));
            String::new()
        });
        user.phone = encrypt_string(user.phone, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt phone in delete_user due to {:?}",
                e
            ));
            String::new()
        });
        user.address = encrypt_string(user.address, &code).unwrap_or_else(|e| {
            log(format!(
                "Could not encrypt address in delete_user due to {:?}",
                e
            ));
            String::new()
        });

        user.devices = vec![];
        user.roles = vec![];
        user.ratings = vec![];
        user.office_ids = vec![];
        user.deleted = true;
        user.modified = Utc::now();
        Ok(user)
    })
    .await?;

    // Hard delete auth email entry.
    cosmos_utils::delete(AUTH_EMAIL_COLLECTION, [&email], &email, None).await?;
    cosmos_utils::delete(AUTH_NID_COLLECTION, [&nid], &nid, None).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_user),
        extra: None::<Empty>,
    }))
}
