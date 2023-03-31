use crate::fault::Fault;
use crate::models::{Ad, Claims, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::AD_COLLECTION;
use chrono::Utc;
use cosmos_utils::{get, upload_image, upsert};
use warp::filters::multipart::FormData;
use warp::reject;

pub async fn ad_image_put(
    id: String,
    claims: Claims,
    _v: u8,
    f: FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (mut ad, etag): (Ad, _) = get(AD_COLLECTION, [&id], &id).await?;

    if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    let image_id = upload_image(f).await?;
    ad.image = Some(image_id);
    ad.modified = Utc::now();

    upsert(AD_COLLECTION, [&id], &ad, Some(&etag)).await?;

    // TODO: Delete old image, if any.
    Ok(warp::reply::json(&DataResponse {
        data: Some(ad),
        extra: None::<Empty>,
    }))
}
