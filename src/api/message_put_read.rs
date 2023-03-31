use crate::fault::Fault;
use crate::models::{Bid, Claims, Message, Task};
use crate::util::{DataResponse, Empty};
use crate::{BID_COLLECTION, MESSAGE_COLLECTION, TASK_COLLECTION};
use chrono::Utc;
use cosmos_utils::{get, modify};
use tokio::join;
use warp::reject;

pub async fn message_put_read(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
    message_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Need to get the task and the bid in order to know which of the chatting users is allowed to
    // mark this message as read
    let (task, bid) = join!(
        get(TASK_COLLECTION, [&office_id], &task_id),
        get(BID_COLLECTION, [&office_id], &bid_id)
    );
    let (task, _): (Task, _) = task?;
    let (bid, _): (Bid, _) = bid?;
    let message = modify(
        MESSAGE_COLLECTION,
        [&office_id],
        &message_id,
        |mut message: Message| {
            if message.office_id != office_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "office_id does not match url ({} != {}).",
                    message.office_id, office_id
                ))));
            }
            if message.task_id != task_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "task_id does not match url ({} != {}).",
                    message.task_id, task_id
                ))));
            }
            if message.bid_id != bid_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "bid_id does not match url ({} != {}).",
                    message.bid_id, bid_id
                ))));
            }
            if message.chat_id != chat_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "chat_id does not match url ({} != {}).",
                    message.chat_id, chat_id
                ))));
            }

            if &task.user_id == &message.user_id {
                if &claims.sub != &bid.craftsman_id {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "Only the receiver is allowed to mark message as read",
                    ))));
                }
            } else {
                if &claims.sub != &task.user_id {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "Only the receiver is allowed to mark message as read",
                    ))));
                }
            }

            message.is_read = true;
            message.modified = Utc::now();
            Ok(message)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(message),
        extra: None::<Empty>,
    }))
}
