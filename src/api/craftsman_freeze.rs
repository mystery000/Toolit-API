use crate::fault::Fault;
use crate::models::{Claims, Craftsman, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::CRAFTSMAN_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn craftsman_freeze(
    office_id: String,
    craftsman_id: String,
    r: DataRequest<bool, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let freeze;
    if let Some(q) = r.data {
        freeze = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    let craftsman = modify(
        CRAFTSMAN_COLLECTION,
        [&office_id],
        &craftsman_id,
        |mut craftsman: Craftsman| {
            if !has_role(
                Some(&craftsman.office_id),
                &claims,
                RoleFlags::OFFICE_CONTENT_ADMIN,
            ) {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "Caller is not a content admin for the office of the craftsman"
                ))));
            }
            craftsman.frozen = freeze;
            craftsman.modified = Utc::now();
            Ok(craftsman)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(craftsman),
        extra: None::<Empty>,
    }))
}
