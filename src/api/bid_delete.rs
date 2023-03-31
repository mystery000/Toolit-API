use crate::fault::Fault;
use crate::models::{Bid, Claims, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::BID_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn bid_delete(
    office_id: String,
    task_id: String,
    bid_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let deleted_bid = modify(BID_COLLECTION, [&office_id], &bid_id, |mut bid: Bid| {
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

        if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
            && &bid.craftsman_id != &claims.sub
        {
            return Err(reject::custom(Fault::Forbidden(format!(
                "User does not have sufficient roles."
            ))));
        }
        bid.deleted = true;
        bid.modified = Utc::now();
        Ok(bid)
    })
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_bid),
        extra: None::<Empty>,
    }))
}
