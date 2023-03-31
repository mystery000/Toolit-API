use crate::fault::Fault;
use crate::models::{Bid, Claims, Message, Task};
use crate::util::{DataResponse, Empty};
use crate::{BID_COLLECTION, MESSAGE_COLLECTION, TASK_COLLECTION};
use chrono::Utc;
use cosmos_utils::{get, query_crosspartition_etag, CosmosSaga};
use tokio::join;
use warp::reject;

pub async fn chat_put_read(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
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

    let message_user_id = if &task.user_id == &claims.sub {
        bid.craftsman_id
    } else if &bid.craftsman_id == &claims.sub {
        task.user_id
    } else {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User is neither task owner nor craftsman",
        ))));
    };

    let q = format!(
        r#"SELECT * FROM {} m WHERE m.chatId = "{}" AND (NOT IS_DEFINED(m.isRead) OR m.isRead = false) AND m.userId = "{}""#,
        MESSAGE_COLLECTION, chat_id, message_user_id
    );
    // NOTE: Below call not actually cross partition since we pass false, this function should perhaps
    // be split up
    let messages: Vec<(Message, _)> =
        query_crosspartition_etag(MESSAGE_COLLECTION, [&office_id], q, -1, false).await?;
    let mut saga = CosmosSaga::new();
    let mut msgs = vec![];
    for (mut message, etag) in messages {
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

        message.is_read = true;
        message.modified = Utc::now();
        saga.upsert(
            MESSAGE_COLLECTION,
            [&office_id],
            &message,
            &message.id,
            Some(&etag),
        )
        .await?;
        msgs.push(message);
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(msgs),
        extra: None::<Empty>,
    }))
}
