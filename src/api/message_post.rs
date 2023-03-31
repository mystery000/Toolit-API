use crate::fault::Fault;
use crate::models::{Bid, Claims, Craftsman, Message, Task, User, Chat};
use crate::push::send_custom_pn;
use crate::util::{log, DataRequest, DataResponse, Empty};
use crate::{
    BID_COLLECTION, CRAFTSMAN_COLLECTION, MESSAGE_COLLECTION, NOTIFICATION_HUB_ACCOUNT,
    TASK_COLLECTION, USER_COLLECTION, CHAT_COLLECTION
};
use cosmos_utils::{get, insert};
use uuid::Uuid;
use warp::reject;

pub async fn message_post(
    office_id: String,
    task_id: String,
    bid_id: String,
    chat_id: String,
    r: DataRequest<Message, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut message;
    if let Some(q) = r.data {
        message = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }
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

    let task_f = get(TASK_COLLECTION, [&office_id], &task_id);
    let bid_f = get(BID_COLLECTION, [&office_id], &bid_id);
    // FIXME(J): Extra cosmos call which just makes sure that the chat is correct, can be removed
    // after correctness is proven
    let chat_f = get(CHAT_COLLECTION, [&office_id], &chat_id);
    let (task_r, bid_r, chat_r) = tokio::join!(task_f, bid_f, chat_f);
    let (task, _): (Task, _) = task_r?;
    let (bid, _): (Bid, _) = bid_r?;
    let (chat, _): (Chat, _) = chat_r?;
    if chat.task_id != task_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "chats task id does not match url ({} != {}).",
            chat.task_id, task_id
        ))));
    }

    if task.user_id != claims.sub && bid.craftsman_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    message.id = Uuid::new_v4().to_string();
    message.is_read = false;
    message.modified = chrono::Utc::now();

    insert(MESSAGE_COLLECTION, [&office_id], &message, None).await?;

    // Send a PN to the receiver in a separate thread
    tokio::task::spawn(async move {
        let (cm_r, to_r) = tokio::join!(
            get(CRAFTSMAN_COLLECTION, [&office_id], &bid.craftsman_id),
            get(USER_COLLECTION, [&task.user_id], &task.user_id)
        );
        let (craftsman, _): (Craftsman, _) = match cm_r {
            Ok(r) => r,
            Err(e) => {
                log(format!(
                    "Could not send PN in message_post due to not being able to get CM. Err: {}",
                    e
                ));
                return ();
            }
        };
        let (craftsman, _): (User, _) = match get(
            USER_COLLECTION,
            [&craftsman.user_id],
            &craftsman.user_id,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                log(format!("Could not send PN in message_post due to not being able to get CM user. Err: {}", e));
                return ();
            }
        };
        let (task_owner, _): (User, _) = match to_r {
            Ok(r) => r,
            Err(e) => {
                log(format!("Could not send PN in message_post due to not being able to get TO user. Err: {}", e));
                return ();
            }
        };

        if task.user_id == claims.sub {
            // If the receiver is the craftsman
            if let Err(e) = send_custom_pn(
                &craftsman,
                &format!("{} skickade dig ett meddelande.", task_owner.name()),
                None,
                &NOTIFICATION_HUB_ACCOUNT,
            )
            .await
            {
                log(format!("Could not send PN in message_post due to {}", e));
            }
        } else {
            // If the receiver is the task-owner
            if let Err(e) = send_custom_pn(
                &task_owner,
                &format!("{} skickade dig ett meddelande.", craftsman.name()),
                None,
                &NOTIFICATION_HUB_ACCOUNT,
            )
            .await
            {
                log(format!("Could not send PN in message_post due to {}", e));
            }
        }
    });

    Ok(warp::reply::json(&DataResponse {
        data: Some(&message),
        extra: None::<Empty>,
    }))
}
