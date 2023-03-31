use crate::fault::Fault;
use crate::models::{Claims, PublishStatus, RoleFlags, Task};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::TASK_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn task_status_put(
    office_id: String,
    task_id: String,
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

    let task = modify(
        TASK_COLLECTION,
        [&office_id],
        &task_id,
        |mut task: Task| match publish_status {
            PublishStatus::Published => {
                match task.publish_status {
                    PublishStatus::Published => {
                        if claims.sub != task.user_id {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                    PublishStatus::Unpublished => {
                        if claims.sub != task.user_id {
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
                task.modified = Utc::now();
                task.publish_status = publish_status.clone();
                Ok(task)
            }
            PublishStatus::Unpublished => {
                match task.publish_status {
                    PublishStatus::Published => {
                        if claims.sub != task.user_id
                            && !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN)
                        {
                            return Err(reject::custom(Fault::Forbidden(format!(
                                "User does not have sufficient roles."
                            ))));
                        }
                    }
                    PublishStatus::Unpublished => {
                        if claims.sub != task.user_id {
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
                task.modified = Utc::now();
                task.publish_status = publish_status.clone();
                Ok(task)
            }
            PublishStatus::Flagged => {
                if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "User does not have sufficient roles."
                    ))));
                }
                task.modified = Utc::now();
                task.publish_status = publish_status.clone();
                Ok(task)
            }
        },
    )
    .await?;

    // TODO: Delete old image, if any.
    Ok(warp::reply::json(&DataResponse {
        data: Some(task),
        extra: None::<Empty>,
    }))
}
