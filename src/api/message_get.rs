use crate::fault::Fault;
use crate::models::{Claims, Message, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::MESSAGE_COLLECTION;
use cosmos_utils::get;
use warp::reject;

pub async fn message_get(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
    message_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "This endpoint is only avaliable to office content admins.",
        ))));
    }

    let (message, _etag): (Message, _) = get(MESSAGE_COLLECTION, [&office_id], &message_id).await?;
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

    if message.id != message_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "message_id does not match url ({} != {}).",
            message.id, message_id
        ))));
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(message),
        extra: None::<Empty>,
    }))
}
