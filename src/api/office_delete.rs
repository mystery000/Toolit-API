use crate::fault::Fault;
use crate::models::{Claims, Office, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::OFFICE_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn office_delete(
    office_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let deleted_office = modify(
        OFFICE_COLLECTION,
        [&office_id],
        &office_id,
        |mut office: Office| {
            if office.id != office_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "office_id does not match url ({} != {}).",
                    office.id, office_id
                ))));
            }

            if !has_role(None::<&str>, &claims, RoleFlags::GLOBAL_CONTENT_ADMIN)
                && !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
            {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "User does not have sufficient roles."
                ))));
            }
            office.deleted = true;
            office.modified = Utc::now();
            Ok(office)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_office),
        extra: None::<Empty>,
    }))
}
