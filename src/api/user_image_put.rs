use crate::fault::Fault;
use crate::models::{Claims, User};
use crate::util::{DataResponse, Empty};
use crate::USER_COLLECTION;
use chrono::Utc;
use cosmos_utils::{get, upload_image, upsert};
use warp::filters::multipart::FormData;
use warp::reject;

pub async fn user_image_put(
    id: String,
    claims: Claims,
    _v: u8,
    f: FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (mut user, etag): (User, _) = get(USER_COLLECTION, [&id], &id).await?;

    if claims.sub != id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the put user ({} != {})",
            claims.sub, id
        ))));
    }

    let image_id = upload_image(f).await?;
    user.images.push(image_id);
    user.modified = Utc::now();

    upsert(USER_COLLECTION, [&id], &user, Some(&etag)).await?;

    // TODO: Delete old image, if any.
    Ok(warp::reply::json(&DataResponse {
        data: Some(user),
        extra: None::<Empty>,
    }))
}
