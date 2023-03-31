use crate::fault::Fault;
use crate::models::{Claims, Task, User};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::{TASK_COLLECTION, USER_COLLECTION};
use cosmos_utils::{insert, modify};
use uuid::Uuid;
use warp::reject;

pub async fn task_post(
    office_id: String,
    r: DataRequest<Task, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut task;
    if let Some(q) = r.data {
        task = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if task.user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Only allowed to post your own tasks ({} != {}).",
            task.user_id, claims.sub
        ))));
    }

    if task.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            task.office_id, office_id
        ))));
    }

    task.id = Uuid::new_v4().to_string();
    task.accepted_bid = None;
    task.finished = false;
    task.craftsman_indicated_finished = false;
    task.rated = false;
    task.modified = chrono::Utc::now();

    insert(TASK_COLLECTION, [&office_id], &task, None).await?;

    // Insert this office as relevant to the user
    // TODO(Jonathan): The office will never stop being relevant to the user, perhaps we should add
    // that?
    // FIXME(Jonathan): If this modify fails then we should likely back off from the insert as
    // well since we then would have created an orphan task without registering the user as
    // interested in the office
    modify(
        USER_COLLECTION,
        [&task.user_id],
        &task.user_id,
        |mut user: User| {
            // Only register interest if the office id is not already registered
            if !user.office_ids.contains(&office_id) {
                user.office_ids.push(office_id.clone());
            }
            Ok(user)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&task),
        extra: None::<Empty>,
    }))
}
