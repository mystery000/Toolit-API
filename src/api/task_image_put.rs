use crate::fault::Fault;
use crate::models::{Claims, Task};
use crate::util::{DataResponse, Empty};
use crate::TASK_COLLECTION;
use chrono::Utc;
use cosmos_utils::{get, upload_image, upsert};
use warp::filters::multipart::FormData;
use warp::reject;

pub async fn task_image_put(
    office_id: String,
    task_id: String,
    claims: Claims,
    _v: u8,
    f: FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (mut task, etag): (Task, _) = get(TASK_COLLECTION, [&office_id], &task_id).await?;
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

    if claims.sub != task.user_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    let image_id = upload_image(f).await?;
    task.images.push(image_id);
    task.modified = Utc::now();

    upsert(TASK_COLLECTION, [&office_id], &task, Some(&etag)).await?;

    // TODO: Delete old image, if any.
    Ok(warp::reply::json(&DataResponse {
        data: Some(task),
        extra: None::<Empty>,
    }))
}
