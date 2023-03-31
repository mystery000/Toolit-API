use crate::fault::Fault;
use crate::models::{Bid, Claims, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::BID_COLLECTION;
use cosmos_utils::get;
use warp::reject;

pub async fn bid_get(
    office_id: String,
    task_id: String,
    bid_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (bid, _etag): (Bid, _) = get(BID_COLLECTION, [&office_id], &bid_id).await?;
    if bid.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            bid.office_id, office_id
        ))));
    }
    if bid.task_id != task_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "task_id does not match url ({} != {}).",
            bid.task_id, task_id
        ))));
    }

    if bid.id != bid_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "bid_id does not match url ({} != {}).",
            bid.id, bid_id
        ))));
    }

    //TODO (Jonathan): Make sure the has_role works for craftsmen
    if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
        && &bid.craftsman_id != &claims.sub
    {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(bid),
        extra: None::<Empty>,
    }))
}
