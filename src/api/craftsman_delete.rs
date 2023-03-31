use crate::fault::Fault;
use crate::models::{Claims, Craftsman, RoleFlags, User};
use crate::util::{has_role, DataResponse, Empty};
use crate::{CRAFTSMAN_COLLECTION, USER_COLLECTION};
use chrono::Utc;
use cosmos_utils::CosmosSaga;
use warp::reject;

pub async fn craftsman_delete(
    office_id: String,
    craftsman_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut saga = CosmosSaga::new();
    let deleted_craftsman = saga
        .modify(
            CRAFTSMAN_COLLECTION,
            [&office_id],
            &craftsman_id,
            |mut craftsman: Craftsman| async {
                if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
                    && claims.sub != craftsman.user_id
                {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "User does not have sufficient roles."
                    ))));
                }
                craftsman.deleted = true;
                craftsman.modified = Utc::now();
                Ok(craftsman)
            },
        )
        .await?;
    // NOTE: The craftmans ID is the same as it's users
    saga.modify(
        USER_COLLECTION,
        [&craftsman_id],
        &craftsman_id,
        |mut user: User| async {
            let sub = format!("{} {}", office_id, craftsman_id);
            let mut found = None;
            // Remove the role from the user
            for (i, role) in user.roles.iter().enumerate() {
                if role.flg.intersects(RoleFlags::CRAFTSMAN) && role.sub.as_ref() == Some(&sub) {
                    found = Some(i);
                    break;
                }
            }
            if let Some(i) = found {
                user.roles.remove(i);
            }
            Ok(user)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_craftsman),
        extra: None::<Empty>,
    }))
}
