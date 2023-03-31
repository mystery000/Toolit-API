use crate::fault::Fault;
use crate::models::{Bid, Claims, Craftsman, Message, PublishStatus, RoleFlags, Task, User};
use crate::push::silent_push;
use crate::util::{has_role, log, DataRequest, DataResponse, Empty};
use crate::{
    BID_COLLECTION, CRAFTSMAN_COLLECTION, MESSAGE_COLLECTION, NOTIFICATION_HUB_ACCOUNT,
    TASK_COLLECTION, USER_COLLECTION,
};
use chrono::Utc;
use cosmos_utils::{get, modify};
use warp::reject;

pub async fn message_status_put(
    office_id: String,
    _task_id: String,
    _bid_id: String,
    _chat_id: String,
    message_id: String,
    r: DataRequest<PublishStatus, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let publish_status = match r.data {
        Some(p) => p,
        None => {
            return Err(reject::custom(Fault::NoData));
        }
    };

    let message = modify(
        MESSAGE_COLLECTION,
        [&office_id],
        &message_id,
        |mut message: Message| match publish_status {
            PublishStatus::Published => {
                match message.publish_status {
                    PublishStatus::Published => {
                        if claims.sub != message.user_id {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                    PublishStatus::Unpublished => {
                        if claims.sub != message.user_id {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                    PublishStatus::Flagged => {
                        if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                }
                message.modified = Utc::now();
                message.publish_status = publish_status.clone();
                Ok(message)
            }
            PublishStatus::Unpublished => {
                match message.publish_status {
                    PublishStatus::Published => {
                        if claims.sub != message.user_id
                            && !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
                        {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                    PublishStatus::Unpublished => {
                        if claims.sub != message.user_id {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                    PublishStatus::Flagged => {
                        if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                }
                message.modified = Utc::now();
                message.publish_status = publish_status.clone();
                Ok(message)
            }
            PublishStatus::Flagged => {
                if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "User does not have sufficient roles."
                    ))));
                }
                message.modified = Utc::now();
                message.publish_status = publish_status.clone();
                Ok(message)
            }
        },
    )
    .await?;

    // Send a PN to the receiver
    let (task, _): (Task, _) = get(TASK_COLLECTION, [&message.office_id], &message.task_id).await?;
    if task.user_id == message.user_id {
        // If the receiver is the craftsman
        let (bid, _): (Bid, _) = get(BID_COLLECTION, [&message.office_id], &message.bid_id).await?;

        let (craftsman, _): (Craftsman, _) =
            get(CRAFTSMAN_COLLECTION, [&bid.office_id], &bid.craftsman_id).await?;

        let (user, _): (User, _) =
            get(USER_COLLECTION, [&craftsman.user_id], &craftsman.user_id).await?;

        if let Err(e) = silent_push(&user, None, &NOTIFICATION_HUB_ACCOUNT).await {
            log(format!("Could not send PN in message_post due to {}", e));
        }
    } else {
        // If the receiver is the task-owner
        let (user, _): (User, _) = get(USER_COLLECTION, [&task.user_id], &task.user_id).await?;
        if let Err(e) = silent_push(&user, None, &NOTIFICATION_HUB_ACCOUNT).await {
            log(format!("Could not send PN in message_post due to {}", e));
        }
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(message),
        extra: None::<Empty>,
    }))
}
