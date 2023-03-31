use crate::fault::Fault;
use crate::models::{Ad, Bid, Chat, Claims, Craftsman, Message, Office, Payment, Task, User};
use crate::util::{self, DataResponse, Empty};
use crate::{
    AD_COLLECTION, BID_COLLECTION, CHAT_COLLECTION, CRAFTSMAN_COLLECTION, MESSAGE_COLLECTION,
    OFFICE_COLLECTION, PAYMENT_COLLECTION, TASK_COLLECTION, USER_COLLECTION,
};
use chrono::{DateTime, Utc};
use cosmos_utils::{get, query, query_crosspartition};
use futures::future::join_all;
use serde::Serialize;
use warp::{
    http::{header, Response},
    reject,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPollDataResponse<'a> {
    #[serde(skip_serializing_if = "util::is_none")]
    pub user: Option<&'a User>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub offices: Vec<Office>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub tasks: Vec<Task>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub craftsmen: Vec<Craftsman>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub chats: Vec<Chat>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub payments: Vec<Payment>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub bids: Vec<Bid>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub ads: Vec<Ad>,
}

/// Poll for residents, returns information about all aspects of a user.
pub async fn user_poll(
    user_id: String,
    claims: Claims,
    _v: u8,
    _range: u16,
    since: Option<DateTime<Utc>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User id does not match signed in user ({} != {}).",
            user_id, claims.sub
        ))));
    }

    let (user, _etag): (User, _) = get(USER_COLLECTION, [&user_id], user_id.clone()).await?;

    // Get all offices we are registered in
    let offices_futs = user.office_ids.iter().map(|id| {
        // Create futures for each cosmos call.
        async move {
            let (office, _): (Office, _) = get(OFFICE_COLLECTION, [&id], &id).await?;
            let r: Result<Office, warp::Rejection> = Ok(office);
            r
        }
    });
    let office_futs = join_all(offices_futs);

    // Get all craftsmen in offices we are registered in
    let craftsmen_futs = user.office_ids.iter().map(|id| {
        // Create futures for each cosmos call.
        let q = format!(r#"SELECT * FROM {}"#, CRAFTSMAN_COLLECTION);
        async move {
            let craftsmen: Vec<Craftsman> = query(CRAFTSMAN_COLLECTION, [&id], q, -1).await?;
            Result::<Vec<Craftsman>, warp::Rejection>::Ok(craftsmen)
        }
    });
    let craftsmen_futs = join_all(craftsmen_futs);

    // Get all tasks in offices we are registered in
    let tasks_futs = user.office_ids.iter().map(|id| {
        // Create futures for each cosmos call.
        let q = format!(r#"SELECT * FROM {}"#, TASK_COLLECTION);
        async move {
            let tasks: Vec<Task> = query(TASK_COLLECTION, [&id], q, -1).await?;
            Result::<Vec<Task>, warp::Rejection>::Ok(tasks)
        }
    });
    let task_futs = join_all(tasks_futs);

    // Create futures for each cosmos call.
    let ads_futs = async move {
        let q = format!(r#"SELECT * FROM {}"#, AD_COLLECTION);
        let ads = query_crosspartition(AD_COLLECTION, [()], q, -1, true).await?;
        let r: Result<Vec<Ad>, warp::Rejection> = Ok(ads);
        r
    };

    let (offices, tasks, craftsmen, ads) =
        tokio::join!(office_futs, task_futs, craftsmen_futs, ads_futs);
    let mut ads: Vec<Ad> = ads?;
    let mut offices: Vec<_> = offices
        .into_iter()
        .filter(|u| u.is_ok())
        .map(|u| u.unwrap())
        .collect();
    let tasks_iter = tasks.into_iter().filter(|u| u.is_ok()).map(|u| u.unwrap());
    let mut tasks = vec![];
    for b in tasks_iter {
        tasks.extend(b);
    }
    let craftsmen_iter = craftsmen
        .into_iter()
        .filter(|u| u.is_ok())
        .map(|u| u.unwrap());
    let mut craftsmen = vec![];
    for b in craftsmen_iter {
        craftsmen.extend(b);
    }

    // NOTE: In order to only get the bids that we are a part of we are using the IN SQL query,
    // this requires that we format our list of ids like: ("task_id1", "task_id2") and that's what
    // this code does.
    // We create two lists, one for each craftsman ID that we possess and one for each task ID that
    // we have created. If a bid has a task id or a craftsman id that exists in either list then we
    // return that bid as being one we are part of.
    let mut my_task_ids = String::new();
    for task in &tasks {
        if task.user_id == user.id {
            if my_task_ids.is_empty() {
                my_task_ids.push_str(&format!(r#""{}""#, task.id));
            } else {
                my_task_ids.push_str(&format!(r#","{}""#, task.id));
            }
        }
    }

    // NOTE: An empty list can not be described by (), but can be described by ("")
    let my_task_ids = if my_task_ids.len() > 0 {
        format!("({})", my_task_ids)
    } else {
        format!(r#"("")"#)
    };
    let mut my_craftsmen_ids = String::new();
    for craftsman in &craftsmen {
        if craftsman.user_id == user.id {
            if my_craftsmen_ids.is_empty() {
                my_craftsmen_ids.push_str(&format!(r#""{}""#, craftsman.id));
            } else {
                my_craftsmen_ids.push_str(&format!(r#","{}""#, craftsman.id));
            }
        }
    }

    // NOTE: An empty list can not be described by (), but can be described by ("")
    let my_craftsmen_ids = if my_craftsmen_ids.len() > 0 {
        format!("({})", my_craftsmen_ids)
    } else {
        format!(r#"("")"#)
    };
    // NOTE: Get only the payments where user was the task creator or user is craftsman
    let bids_futs = user.office_ids.iter().map(|id| {
        let q = format!(
            r#"SELECT * FROM {} u WHERE u.craftsmanId IN {} OR u.taskId IN {}"#,
            BID_COLLECTION, my_craftsmen_ids, my_task_ids
        );
        async move {
            let bids: Vec<Bid> = query(BID_COLLECTION, [&id], q, -1).await?;
            Result::<Vec<Bid>, warp::Rejection>::Ok(bids)
        }
    });
    let bids_futs = join_all(bids_futs);

    // NOTE: Get only the payments where user was the task creator or user is craftsman
    let payments_futs = user.office_ids.iter().map(|id| {
        // Create futures for each cosmos call.
        let q = format!(
            r#"SELECT * FROM {} u WHERE u.craftsmanId IN {} OR u.taskId IN {}"#,
            PAYMENT_COLLECTION, my_craftsmen_ids, my_task_ids
        );
        async move {
            let payments: Vec<Payment> = query(PAYMENT_COLLECTION, [&id], q, -1).await?;
            Result::<Vec<Payment>, warp::Rejection>::Ok(payments)
        }
    });
    let payments_futs = join_all(payments_futs);

    let (bids, payments) = tokio::join!(bids_futs, payments_futs,);

    let bids_iter = bids.into_iter().filter(|u| u.is_ok()).map(|u| u.unwrap());
    let mut bids = vec![];
    for b in bids_iter {
        bids.extend(b);
    }

    let payments_iter = payments
        .into_iter()
        .filter(|u| u.is_ok())
        .map(|u| u.unwrap());
    let mut payments = vec![];
    for b in payments_iter {
        payments.extend(b);
    }

    // NOTE: Only get chats from bids that we are part of
    let chats_futs: Vec<_> = bids
        .iter()
        .map(|bid| {
            let id = &bid.id;
            // Create futures for each cosmos call.
            let q = format!(
                r#"SELECT * FROM {} u WHERE u.bidId = "{}""#,
                CHAT_COLLECTION, id
            );
            async move {
                let chats: Vec<Chat> = query(CHAT_COLLECTION, [&bid.office_id], q, -1).await?;
                Result::<Vec<Chat>, warp::Rejection>::Ok(chats)
            }
        })
        .collect();
    let chats = join_all(chats_futs).await;

    let chats_iter = chats.into_iter().filter(|u| u.is_ok()).map(|u| u.unwrap());
    let mut chats = vec![];
    for b in chats_iter {
        chats.extend(b);
    }

    // NOTE: Only get messages from bids that we are part of
    let messages_futs: Vec<_> = chats
        .iter()
        .map(|chat| {
            let id = &chat.id;
            // Create futures for each cosmos call.
            let q = format!(
                r#"SELECT * FROM {} u WHERE u.chatId = "{}""#,
                MESSAGE_COLLECTION, id
            );
            async move {
                let messages: Vec<Message> =
                    query(MESSAGE_COLLECTION, [&chat.office_id], q, -1).await?;
                Result::<Vec<Message>, warp::Rejection>::Ok(messages)
            }
        })
        .collect();
    let messages = join_all(messages_futs).await;

    let messages_iter = messages
        .into_iter()
        .filter(|u| u.is_ok())
        .map(|u| u.unwrap());
    let mut messages = vec![];
    for b in messages_iter {
        messages.extend(b);
    }

    let mut new_user = Some(&user);
    // Remove all of the entries that have not been updated since `since`.
    // TODO: This filtering could be done in the queries rather than after getting them from the
    // database
    if let Some(since) = since {
        new_user = if user.modified >= since {
            Some(&user)
        } else {
            None
        };
        offices = offices
            .into_iter()
            .filter(|u| u.modified >= since)
            .collect();
        tasks = tasks.into_iter().filter(|u| u.modified >= since).collect();
        craftsmen = craftsmen
            .into_iter()
            .filter(|u| u.modified >= since)
            .collect();
        messages = messages
            .into_iter()
            .filter(|u| u.modified >= since)
            .collect();
        chats = chats.into_iter().filter(|u| u.modified >= since).collect();
        payments = payments
            .into_iter()
            .filter(|u| u.modified >= since)
            .collect();
        bids = bids.into_iter().filter(|u| u.modified >= since).collect();
        ads = ads.into_iter().filter(|u| u.modified >= since).collect();
    }
    let user = new_user;

    let res = match serde_json::to_string(&DataResponse {
        data: Some(&UserPollDataResponse {
            user,
            offices,
            tasks,
            craftsmen,
            messages,
            chats,
            payments,
            bids,
            ads,
        }),
        extra: None::<Empty>,
    }) {
        Ok(g) => g,
        Err(err) => {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not serialize response into json: {}.",
                err.to_string()
            ))));
        }
    };

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::LAST_MODIFIED,
            format!("{}", Utc::now().format("%a, %d %b %Y %H:%M:%S GMT")),
        )
        .body(res))
}
