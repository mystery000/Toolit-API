use crate::fault::Fault;
use crate::models::{Bid, Claims, Task};
use crate::util::{DataResponse, Empty};
use crate::{BID_COLLECTION, TASK_COLLECTION};
use cosmos_utils::modify;
use serde::Serialize;
use warp::reject;

#[derive(Serialize)]
struct Response {
    bid: Bid,
    task: Task,
}

pub async fn bid_cancel(
    office_id: String,
    task_id: String,
    bid_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let task = modify(TASK_COLLECTION, [&office_id], &task_id, |mut task: Task| {
        if claims.sub != task.user_id {
            return Err(reject::custom(Fault::Forbidden(format!(
                "Only the task poster may cancel bids",
            ))));
        }
        if task.finished {
            return Err(reject::custom(Fault::Forbidden(format!(
                "May not cancel bids for finished tasks",
            ))));
        }
        if let Some(accepted_bid) = task.accepted_bid {
            if accepted_bid != bid_id {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "Task has not accepted a bid from {}",
                    bid_id
                ))));
            }
        } else {
            return Err(reject::custom(Fault::Forbidden(format!(
                "Task has not accepted a bid from {}",
                bid_id
            ))));
        }

        task.accepted_bid = None;
        Ok(task)
    })
    .await?;

    let bid = match modify(BID_COLLECTION, [&office_id], &bid_id, |mut bid: Bid| {
        bid.is_cancelled = true;
        Ok(bid)
    })
    .await
    {
        Ok(r) => r,
        Err(e) => {
            // TODO: we need to reverse the previous change in the task here
            return Err(e.into());
        }
    };

    Ok(warp::reply::json(&DataResponse {
        data: Some(&Response { bid, task }),
        extra: None::<Empty>,
    }))
}
