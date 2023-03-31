use crate::fault::Fault;
use crate::models::{Claims, Message};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::MESSAGE_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn message_put(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
    message_id: String,
    r: DataRequest<Message, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_message;
    if let Some(q) = r.data {
        new_message = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if office_id != new_message.office_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Office id does not match the url {} != {}",
            office_id, new_message.office_id
        ))));
    }

    if task_id != new_message.task_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "task id does not match the url {} != {}",
            task_id, new_message.task_id
        ))));
    }

    if bid_id != new_message.bid_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "bid id does not match the url {} != {}",
            bid_id, new_message.bid_id
        ))));
    }

    if chat_id != new_message.chat_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "chat id does not match the url {} != {}",
            chat_id, new_message.chat_id
        ))));
    }

    if message_id != new_message.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted message id is not the same as the url {} != {}",
            message_id, new_message.id
        ))));
    }

    if new_message.user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the poster of the message"
        ))));
    }

    let message = modify(
        MESSAGE_COLLECTION,
        [&office_id],
        &message_id,
        |mut message: Message| {
            message.text = new_message.text.clone();
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
