use crate::fault::Fault;
use crate::models::{Claims, Task};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::TASK_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn task_put(
    office_id: String,
    task_id: String,
    r: DataRequest<Task, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_task;
    if let Some(q) = r.data {
        new_task = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if office_id != new_task.office_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Office id does not match the url {} != {}",
            office_id, new_task.office_id
        ))));
    }

    if task_id != new_task.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted task id is not the same as the url {} != {}",
            task_id, new_task.id
        ))));
    }

    if new_task.user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the poster of the task"
        ))));
    }

    let task = modify(TASK_COLLECTION, [&office_id], &task_id, |mut task: Task| {
        task.crafts = new_task.crafts.clone();
        task.address = new_task.address.clone();
        task.city = new_task.city.clone();
        task.postcode = new_task.postcode.clone();
        task.date_done = new_task.date_done.clone();
        task.description = new_task.description.clone();
        task.title = new_task.title.clone();
        task.modified = Utc::now();
        Ok(task)
    })
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(task),
        extra: None::<Empty>,
    }))
}
