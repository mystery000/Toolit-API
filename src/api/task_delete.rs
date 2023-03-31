use crate::fault::Fault;
use crate::models::{Claims, RoleFlags, Task};
use crate::util::{has_role, DataResponse, Empty};
use crate::TASK_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn task_delete(
    office_id: String,
    task_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let deleted_task = modify(TASK_COLLECTION, [&office_id], &task_id, |mut task: Task| {
        if task.office_id != office_id {
            return Err(reject::custom(Fault::IllegalArgument(format!(
                "office_id does not match url ({} != {}).",
                task.office_id, office_id
            ))));
        }

        if task.id != task_id {
            return Err(reject::custom(Fault::IllegalArgument(format!(
                "task_id does not match url ({} != {}).",
                task.id, task_id
            ))));
        }

        if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
            && claims.sub != task.user_id
        {
            return Err(reject::custom(Fault::Forbidden(format!(
                "User does not have sufficient roles."
            ))));
        }
        task.deleted = true;
        task.modified = Utc::now();
        Ok(task)
    })
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_task),
        extra: None::<Empty>,
    }))
}
