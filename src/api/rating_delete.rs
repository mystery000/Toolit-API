use crate::fault::Fault;
use crate::models::{Claims, Craftsman, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::CRAFTSMAN_COLLECTION;
use chrono::Utc;
use cosmos_utils::{maybe_modify, ModifyReturn};
use warp::reject;

pub async fn rating_delete(
    office_id: String,
    craftsman_id: String,
    rating_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let deleted_rating_task = maybe_modify(
        CRAFTSMAN_COLLECTION,
        [&office_id],
        &craftsman_id,
        |mut craftsman: Craftsman| {
            if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "User does not have sufficient roles."
                ))));
            }
            for rating in &mut craftsman.ratings {
                if rating.id != rating_id {
                    rating.deleted = true;
                    rating.modified = Utc::now();
                    craftsman.modified = Utc::now();
                    return Ok(ModifyReturn::Replace(craftsman));
                }
            }
            return Err(reject::custom(Fault::Forbidden(format!(
                "No rating with id {}",
                rating_id
            ))));
        },
    )
    .await?
    .inner();

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_rating_task),
        extra: None::<Empty>,
    }))
}
