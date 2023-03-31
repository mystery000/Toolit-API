use crate::{
    fault::Fault,
    models::{Bid, Claims, Payment, RoleFlags, Task},
    util::{has_role, DataResponse, Empty},
    BID_COLLECTION, PAYMENT_COLLECTION, TASK_COLLECTION,
};
use cosmos_utils::get;
use warp::reject;

pub async fn payment_get(
    office_id: String,
    task_id: String,
    bid_id: String,
    payment_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (payment, _etag): (Payment, _) = get(PAYMENT_COLLECTION, [&office_id], &payment_id).await?;

    if payment.task_id != task_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "Task id does not match url ({} != {}).",
            payment.task_id, task_id
        ))));
    } else if payment.bid_id != bid_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "Bid id does not match url ({} != {}).",
            payment.bid_id, bid_id
        ))));
    }

    // Get bid.
    let (bid, _): (Bid, _) = get(BID_COLLECTION, [&office_id], &payment.bid_id).await?;

    // Get task.
    let (task, _): (Task, _) = get(TASK_COLLECTION, [&office_id], &bid.task_id).await?;

    // Make sure user is either admin or the paying user.
    if task.user_id != claims.sub
        && !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_BILLING_ADMIN)
    {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(payment),
        extra: None::<Empty>,
    }))
}
