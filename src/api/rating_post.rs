use crate::fault::Fault;
use crate::models::{Bid, Claims, Craftsman, Rating, Task};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::{BID_COLLECTION, CRAFTSMAN_COLLECTION, TASK_COLLECTION};
use cosmos_utils::{get, CosmosSaga};
use uuid::Uuid;
use warp::reject;

pub async fn rating_post(
    office_id: String,
    craftsman_id: String,
    r: DataRequest<Rating, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut rating;
    if let Some(q) = r.data {
        rating = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }
    // Verify that rating has correct fields
    if rating.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            rating.office_id, office_id
        ))));
    }
    if rating.craftsman_id != craftsman_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "craftsman_id does not match url ({} != {}).",
            rating.craftsman_id, craftsman_id
        ))));
    }

    // Make sure the rating pertains to a task which was owned by the rater, that it is unrated,
    // that it is finished and that it has accepted a bid which is owned by the rated craftsman
    let (mut task, task_etag): (Task, _) =
        get(TASK_COLLECTION, [&office_id], &rating.task_id).await?;
    if task.user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Only task owner can rate a task"
        ))));
    }
    if task.rated {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Task already has a rating"
        ))));
    }
    if !task.finished {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Task is not finished"
        ))));
    }
    let bid = if let Some(bid_id) = &task.accepted_bid {
        let (bid, _): (Bid, _) = get(BID_COLLECTION, [&office_id], &bid_id).await?;
        bid
    } else {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Task does not have an accepted bid"
        ))));
    };
    if bid.craftsman_id != rating.craftsman_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Rating does not rate the craftsman of the accepted bid"
        ))));
    }

    // A rating is a number between 1 and 5
    let amount = std::cmp::min(rating.amount, 5);
    let amount = std::cmp::max(amount, 1);
    //The user id of the rating is the rating task owner
    rating.user_id = task.user_id.clone();
    // Insert the rating in the craftsman
    rating.amount = amount;
    rating.id = Uuid::new_v4().to_string();
    rating.created = chrono::Utc::now();
    rating.modified = chrono::Utc::now();

    // We only want to update the craftsman if we can also update the task to have been rated, do
    // this with a saga
    let mut post_saga = CosmosSaga::new();
    let craftsman = post_saga
        .modify(
            CRAFTSMAN_COLLECTION,
            [&office_id],
            &rating.craftsman_id,
            |mut craftsman: Craftsman| async {
                craftsman.ratings.push(rating.clone());
                craftsman.modified = chrono::Utc::now();
                Ok(craftsman)
            },
        )
        .await?;
    task.modified = chrono::Utc::now();
    task.rated = true;
    post_saga
        .upsert(
            TASK_COLLECTION,
            [&office_id],
            &task,
            &task.id,
            Some(&task_etag),
        )
        .await?;
    post_saga.finalize().await;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&craftsman),
        extra: None::<Empty>,
    }))
}
