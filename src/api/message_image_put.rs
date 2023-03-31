use crate::fault::Fault;
use crate::models::{Claims, Message};
use crate::util::{DataResponse, Empty};
use crate::MESSAGE_COLLECTION;
use chrono::Utc;
use cosmos_utils::{get, upload_image, upsert};
use warp::filters::multipart::FormData;
use warp::reject;

pub async fn message_image_put(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
    message_id: String,
    claims: Claims,
    _v: u8,
    f: FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (mut message, etag): (Message, _) =
        get(MESSAGE_COLLECTION, [&office_id], &message_id).await?;
    if task_id != message.task_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "task id does not match the url {} != {}",
            task_id, message.task_id
        ))));
    }

    if bid_id != message.bid_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "bid id does not match the url {} != {}",
            bid_id, message.bid_id
        ))));
    }

    if chat_id != message.chat_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "chat id does not match the url {} != {}",
            chat_id, message.chat_id
        ))));
    }

    if message_id != message.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted message id is not the same as the url {} != {}",
            message_id, message.id
        ))));
    }

    if message.user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the poster of the message"
        ))));
    }

    let image_id = upload_image(f).await?;
    message.image = Some(image_id);
    message.modified = Utc::now();
    upsert(MESSAGE_COLLECTION, [&office_id], &message, Some(&etag)).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(message),
        extra: None::<Empty>,
    }))
}
