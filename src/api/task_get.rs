use crate::fault::Fault;
use crate::models::{Claims, Task};
use crate::util::{DataResponse, Empty};
use crate::TASK_COLLECTION;
use cosmos_utils::get;
use warp::reject;

pub async fn task_get(
    office_id: String,
    task_id: String,
    _claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (task, _etag): (Task, _) = get(TASK_COLLECTION, [&office_id], &task_id).await?;
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

    Ok(warp::reply::json(&DataResponse {
        data: Some(task),
        extra: None::<Empty>,
    }))
}
