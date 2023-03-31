use crate::fault::Fault;
use crate::models::{Bid, Claims, Task, User};
use crate::push::send_custom_pn;
use crate::util::{log, DataResponse, Empty};
use crate::{BID_COLLECTION, NOTIFICATION_HUB_ACCOUNT, TASK_COLLECTION, USER_COLLECTION};
use cosmos_utils::{get, maybe_modify_async, ModifyReturn};

// This endpoint is callable only by the craftsman
pub async fn craftsman_task_finish(
    office_id: String,
    task_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    // TODO(Jonathan): Should be maybe_modify
    let maybe_task = maybe_modify_async(
        TASK_COLLECTION,
        [&office_id],
        &task_id,
        |mut task: Task| async {
            if let Some(accepted_bid) = &task.accepted_bid {
                let (bid, _): (Bid, _) = get(BID_COLLECTION, [&office_id], &accepted_bid).await?;
                if bid.craftsman_id != claims.sub {
                    return Err(warp::reject::custom(Fault::Forbidden(format!(
                        "Only the craftsman of the accepted bid can call this"
                    ))));
                }
            } else {
                return Err(warp::reject::custom(Fault::Forbidden(format!(
                    "Can only call this endpoint if the task has an accepted bid"
                ))));
            }

            if task.craftsman_indicated_finished {
                return Ok(ModifyReturn::DontReplace(task));
            } else {
                task.craftsman_indicated_finished = true;
                return Ok(ModifyReturn::Replace(task));
            }
        },
    )
    .await?;
    let task = match maybe_task {
        ModifyReturn::DontReplace(task) => {
            return Ok(warp::reply::json(&DataResponse {
                data: Some(&task),
                extra: None::<Empty>,
            }));
        }
        ModifyReturn::Replace(task) => task,
    };

    let (user, _): (User, _) = get(USER_COLLECTION, [&task.user_id], &task.user_id).await?;
    // Send PN to the task owner
    if let Err(e) = send_custom_pn(
        &user,
        &format!("Hantverkaren för ett av dina jobb har markerat arbetet som klart. Gör en inspektion och om du håller med så godkänner du jobbet!"),
        None,
        &NOTIFICATION_HUB_ACCOUNT,
    )
    .await
    {
        log(format!("Could not send PN in task_finish due to {}", e));
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(&task),
        extra: None::<Empty>,
    }))
}
