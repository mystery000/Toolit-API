use crate::fault::Fault;
use crate::models::{Bid, Chat, Claims, CraftStatus, Craftsman, RoleFlags, Task, User};
use crate::push::send_custom_pn;
use crate::util::{has_role, log, DataRequest, DataResponse, Empty};
use crate::{
    BID_COLLECTION, CHAT_COLLECTION, CRAFTSMAN_COLLECTION, NOTIFICATION_HUB_ACCOUNT,
    TASK_COLLECTION, USER_COLLECTION,
};
use cosmos_utils::{get, CosmosSaga};
use uuid::Uuid;
use warp::reject;

pub async fn bid_post(
    office_id: String,
    task_id: String,
    r: DataRequest<Bid, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut bid;
    if let Some(q) = r.data {
        bid = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if bid.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            bid.office_id, office_id
        ))));
    }

    if bid.task_id != task_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "task_id does not match url ({} != {}).",
            bid.task_id, task_id
        ))));
    }

    if !has_role(None, &claims, RoleFlags::CRAFTSMAN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Need to be the craftsman in the bid to make it."
        ))));
    }

    // NOTE: This works because we make sure the craftsman id is the userid
    if bid.craftsman_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Only the craftsman is allowed to post a bid."
        ))));
    }

    let (craftsman_r, task_r) = tokio::join!(
        get(CRAFTSMAN_COLLECTION, [&office_id], &bid.craftsman_id),
        get(TASK_COLLECTION, [&office_id], &task_id)
    );
    let (craftsman, _): (Craftsman, _) = craftsman_r?;
    let (task, _): (Task, _) = task_r?;

    if craftsman.frozen {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Frozen craftsmen can not post bids."
        ))));
    }

    // NOTE: Make sure that the craftsman has at least one approved craft that is in the task before he is allowed to post bids
    let mut found = false;
    'outer: for task_craft in &task.crafts {
        for craftsman_craft in &craftsman.crafts {
            if &craftsman_craft.craft_type == task_craft
                && craftsman_craft.status == CraftStatus::Approved
            {
                found = true;
                break 'outer;
            }
        }
    }
    if !found {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Craftsman needs to have at least one approved overlapping craft before he can post bids."
        ))));
    }

    if craftsman.deleted {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Deleted craftsmen can not post bids"
        ))));
    }

    // TODO: We need to set a flag to either use rot och rut or not here
    if !bid.cost_is_correct(task.use_rot_rut) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "The cost of the bid is not correct"
        ))));
    }

    bid.id = Uuid::new_v4().to_string();
    bid.is_cancelled = false;
    bid.modified = chrono::Utc::now();

    //let (bid_insert_r, task_get_r) = tokio::join!(
    //    insert(BID_COLLECTION, [&office_id], &bid, None),
    //    get(TASK_COLLECTION, [&office_id], &task_id)
    //);
    let mut bid_saga = CosmosSaga::new();
    let mut chat_saga = CosmosSaga::new();
    // NOTE: Create a chat for each bid that is created
    let chat = Chat {
        id: Uuid::new_v4().to_string(),
        office_id: office_id.clone(),
        task_id,
        bid_id: bid.id.clone(),
        modified: chrono::Utc::now(),
        deleted: false,
    };
    let (bid_r, chat_r, to_r, bi_r) = tokio::join!(
        bid_saga.insert(BID_COLLECTION, [&office_id], &bid, &bid.id, None),
        chat_saga.insert(CHAT_COLLECTION, [&office_id], &chat, &chat.id, None),
        get(USER_COLLECTION, [&task.user_id], &task.user_id),
        get(USER_COLLECTION, [&claims.sub], &claims.sub)
    );

    // If only one of the chat insertion or bid insertion fails then we revert the other and then
    // return the error
    if let Err(e) = bid_r {
        if chat_r.is_ok() {
            chat_saga.abort().await?;
        }
        return Err(e.into());
    }

    if let Err(e) = chat_r {
        if bid_r.is_ok() {
            bid_saga.abort().await?;
        }
        return Err(e.into());
    }

    // Attempt to send out a PN, do this in a new thread since failure takes a long time and we
    // only log errors, we don't return them.
    tokio::task::spawn(async {
        match to_r {
            Ok(to_r) => {
                match bi_r {
                    Ok(bi_r) => {
                        let (task_owner, _): (User, _) = to_r;
                        let (bidder, _): (User, _) = bi_r;
                        // Send a PN to the task-owner
                        if let Err(e) = send_custom_pn(
                            &task_owner,
                            &format!("{} skapade ett bud på ditt jobb.", bidder.name()),
                            None,
                            &NOTIFICATION_HUB_ACCOUNT,
                        )
                        .await
                        {
                            log(format!("Could not send PN in bid_post due to {}", e));
                        }
                    }
                    Err(e) => {
                        log(format!(
                            "Could not send PN in bid_post due to get failing with bi_r {}",
                            e
                        ));
                    }
                }
            }
            Err(e) => {
                log(format!(
                    "Could not send PN in bid_post due to get failing with to_r {}",
                    e
                ));
            }
        }
    });

    Ok(warp::reply::json(&DataResponse {
        data: Some(&bid),
        extra: None::<Empty>,
    }))
}
