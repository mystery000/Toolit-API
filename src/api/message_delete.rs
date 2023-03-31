use crate::fault::Fault;
use crate::models::{Claims, Message, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::MESSAGE_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn message_delete(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
    message_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let deleted_message = modify(
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

            if message.id != message_id {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "message_id does not match url ({} != {}).",
                    message.id, message_id
                ))));
            }

            if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
                && claims.sub != message.user_id
            {
                return Err(reject::custom(Fault::Forbidden(format!(
                    "User does not have sufficient roles."
                ))));
            }
            message.deleted = true;
            message.modified = Utc::now();
            Ok(message)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_message),
        extra: None::<Empty>,
    }))
}
