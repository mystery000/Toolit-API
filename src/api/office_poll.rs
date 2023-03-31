use crate::fault::Fault;
use crate::models::{
    Bid, Chat, Claims, Craftsman, CraftsmanNote, Message, Office, Payment, RoleFlags, Task, User,
};
use crate::util::{self, has_role, DataResponse, Empty};
use crate::{
    BID_COLLECTION, CHAT_COLLECTION, CRAFTSMAN_COLLECTION, CRAFTSMAN_NOTE_COLLECTION,
    MESSAGE_COLLECTION, OFFICE_COLLECTION, PAYMENT_COLLECTION, TASK_COLLECTION, USER_COLLECTION,
};
use chrono::{DateTime, Utc};
use cosmos_utils::{get, query, query_crosspartition, CosmosErrorStruct};
use serde::Serialize;
use warp::{
    http::{header, Response},
    reject,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPollDataResponse {
    #[serde(skip_serializing_if = "util::is_none")]
    pub office: Option<Office>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub users: Vec<User>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub tasks: Vec<Task>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub craftsmen: Vec<Craftsman>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub craftsman_notes: Vec<CraftsmanNote>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub chats: Vec<Chat>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub payments: Vec<Payment>,
    #[serde(skip_serializing_if = "util::is_empty")]
    pub bids: Vec<Bid>,
}

/// Poll for admins, returns information about an office.
pub async fn office_poll(
    office_id: String,
    claims: Claims,
    _v: u8,
    _range: u16,
    since: Option<DateTime<Utc>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !has_role(
        Some(&office_id),
        &claims,
        RoleFlags::OFFICE_CONTENT_ADMIN
            | RoleFlags::OFFICE_PERSONNEL_ADMIN
            | RoleFlags::OFFICE_BILLING_ADMIN,
    ) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User is not an admin for this office"
        ))));
    }

    let office = async {
        let (office, _): (Office, _) = get(OFFICE_COLLECTION, [&office_id], &office_id).await?;
        if let Some(since) = since {
            if office.modified >= since {
                Result::<_, CosmosErrorStruct>::Ok(Some(office))
            } else {
                Result::<_, CosmosErrorStruct>::Ok(None)
            }
        } else {
            Result::<_, CosmosErrorStruct>::Ok(Some(office))
        }
    };

    let since = match since {
        Some(since) => format!(" AND o._ts >= {}", since.timestamp()),
        None => String::from(""),
    };

    let q = format!(
        "SELECT * FROM {} o WHERE ARRAY_CONTAINS(o.officeIds, \"{}\"){}",
        USER_COLLECTION, &office_id, &since
    );
    let users = async {
        let users: Vec<User> = query_crosspartition(USER_COLLECTION, [()], q, -1, true).await?;
        Result::<_, CosmosErrorStruct>::Ok(users)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        TASK_COLLECTION, &office_id, since
    );
    let tasks = async {
        let tasks: Vec<Task> = query(TASK_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(tasks)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        BID_COLLECTION, &office_id, since
    );
    let bids = async {
        let bids: Vec<Bid> = query(BID_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(bids)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        CHAT_COLLECTION, &office_id, since
    );
    let chats = async {
        let chats: Vec<Chat> = query(CHAT_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(chats)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        MESSAGE_COLLECTION, &office_id, since
    );
    let messages = async {
        let messages: Vec<Message> = query(MESSAGE_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(messages)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        CRAFTSMAN_COLLECTION, &office_id, since
    );
    let craftsmen = async {
        let craftsmen: Vec<Craftsman> = query(CRAFTSMAN_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(craftsmen)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        CRAFTSMAN_NOTE_COLLECTION, &office_id, since
    );
    let craftsman_notes = async {
        let notes: Vec<CraftsmanNote> =
            query(CRAFTSMAN_NOTE_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(notes)
    };

    let q = format!(
        "SELECT * FROM {} o WHERE o.officeId = \"{}\"{}",
        PAYMENT_COLLECTION, &office_id, since
    );
    let payments = async {
        let payments: Vec<Payment> = query(PAYMENT_COLLECTION, [&office_id], q, -1).await?;
        Result::<_, CosmosErrorStruct>::Ok(payments)
    };

    let (office, users, tasks, bids, chats, messages, craftsmen, craftsman_notes, payments) = tokio::join!(
        office,
        users,
        tasks,
        bids,
        chats,
        messages,
        craftsmen,
        craftsman_notes,
        payments
    );
    let office = office?;
    let users = users?;
    let tasks = tasks?;
    let bids = bids?;
    let chats = chats?;
    let messages = messages?;
    let craftsmen = craftsmen?;
    let craftsman_notes = craftsman_notes?;
    let payments = payments?;

    let res = match serde_json::to_string(&DataResponse {
        data: Some(&UserPollDataResponse {
            users,
            office,
            tasks,
            craftsmen,
            craftsman_notes,
            messages,
            chats,
            payments,
            bids,
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
