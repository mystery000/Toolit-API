use crate::fault::Fault;
use crate::models::{Bid, Claims, Task};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::{BID_COLLECTION, TASK_COLLECTION};
use chrono::Utc;
use cosmos_utils::{get, modify_async};
use warp::reject;

pub async fn bid_put(
    office_id: String,
    task_id: String,
    bid_id: String,
    r: DataRequest<Bid, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_bid;
    if let Some(q) = r.data {
        new_bid = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if office_id != new_bid.office_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Office id does not match the url {} != {}",
            office_id, new_bid.office_id
        ))));
    }

    if task_id != new_bid.task_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "task id does not match the url {} != {}",
            task_id, new_bid.task_id
        ))));
    }

    if bid_id != new_bid.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted bid id is not the same as the url {} != {}",
            bid_id, new_bid.id
        ))));
    }

    // NOTE: Craftsman id is the same as the user id
    if new_bid.craftsman_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the poster of the bid"
        ))));
    }

    let bid = modify_async(
        BID_COLLECTION,
        [&office_id],
        &bid_id,
        |mut bid: Bid| async {
            bid.bid_message = new_bid.bid_message.clone();
            let (task, _): (Task, _) = get(TASK_COLLECTION, [&office_id], &bid.task_id).await?;
            // If we want to change the cost of the bid make sure it's correct
            if bid.final_bid != new_bid.final_bid
                || bid.labour_cost != new_bid.labour_cost
                || bid.material_cost != new_bid.material_cost
                || bid.root_deduction != new_bid.root_deduction
            {
                if let Some(accepted_id) = task.accepted_bid {
                    if accepted_id == bid.id {
                        return Err(warp::reject::custom(Fault::Forbidden(String::from(
                            "Can not change cost of a bid that is already accepted",
                        ))));
                    }
                }
                if new_bid.cost_is_correct(task.use_rot_rut) {
                    bid.final_bid = new_bid.final_bid;
                    bid.labour_cost = new_bid.labour_cost;
                    bid.material_cost = new_bid.material_cost;
                    bid.root_deduction = new_bid.root_deduction;
                } else {
                    return Err(warp::reject::custom(Fault::Forbidden(String::from(
                        "The cost of the new bid is not correct",
                    ))));
                }
            }
            bid.modified = Utc::now();
            Ok(bid)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(bid),
        extra: None::<Empty>,
    }))
}
