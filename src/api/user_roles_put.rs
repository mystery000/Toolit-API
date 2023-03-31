use crate::fault::Fault;
use crate::models::{Claims, Role, RoleFlags, User};
use crate::util::{self, DataRequest, DataResponse, Empty};
use crate::USER_COLLECTION;
use cosmos_utils::{get, upsert};
use warp::reject;

/// Assign roles to the user
/// NOTE: This endpoint can not be used to assign a craftsman role to the user, for that you should
/// create a craftsman
pub async fn user_roles_put(
    user_id: String,
    r: DataRequest<Vec<Role>, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let roles;
    if let Some(q) = r.data {
        roles = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    let office_flags = RoleFlags::OFFICE_CONTENT_ADMIN
        | RoleFlags::OFFICE_PERSONNEL_ADMIN
        | RoleFlags::OFFICE_BILLING_ADMIN;

    let global_flags = RoleFlags::GLOBAL_CONTENT_ADMIN
        | RoleFlags::GLOBAL_BILLING_ADMIN
        | RoleFlags::GLOBAL_PERSONNEL_ADMIN;

    for role in &roles {
        if role.flg.intersects(office_flags) {
            if (role.flg - office_flags) != RoleFlags::NONE {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "Cannot mix office roles with other roles."
                ))));
            }

            if let Some(sub) = &role.sub {
                let len = sub.split(' ').collect::<Vec<&str>>().len();
                if len != 1 {
                    return Err(reject::custom(Fault::IllegalArgument(format!(
                        "Wrong number of key components for tenant roles subject ({} != 1).",
                        len,
                    ))));
                } else if !util::has_role(Some(sub), &claims, RoleFlags::OFFICE_PERSONNEL_ADMIN) {
                    if !util::has_role(None, &claims, RoleFlags::GLOBAL_PERSONNEL_ADMIN) {
                        return Err(reject::custom(Fault::Forbidden(format!(
                            "User is not a personnel admin for office {}, or globally.",
                            sub
                        ))));
                    }
                }
            } else {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "Must have subject for office roles."
                ))));
            }
        } else if role.flg.intersects(global_flags) {
            if (role.flg - global_flags) != RoleFlags::NONE {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "Cannot mix global roles with other roles."
                ))));
            } else if let Some(_) = role.sub {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "Cannot have subject for global roles."
                ))));
            }

            if !util::has_role(None, &claims, RoleFlags::GLOBAL_PERSONNEL_ADMIN) {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "User is not a global personnel admin."
                ))));
            }
        } else {
            return Err(reject::custom(Fault::IllegalArgument(format!(
                "Unrecognized role flag {}.",
                role.flg.bits()
            ))));
        }
    }

    let (mut user, etag): (User, _) = get(USER_COLLECTION, [&user_id], user_id.clone()).await?;

    for role in roles {
        user.roles.push(role);
    }

    // Insert the user into the database again.
    upsert(USER_COLLECTION, [&user_id], &user, Some(&etag)).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(user),
        extra: None::<Empty>,
    }))
}
