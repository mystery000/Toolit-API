use crate::fault::Fault;
use crate::models::{Ad, Claims, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::AD_COLLECTION;
use cosmos_utils::insert;
use uuid::Uuid;
use warp::reject;

pub async fn ad_post(
    r: DataRequest<Ad, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut ad;
    if let Some(q) = r.data {
        ad = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    ad.id = Uuid::new_v4().to_string();
    ad.modified = chrono::Utc::now();

    insert(AD_COLLECTION, [&ad.id], &ad, None).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&ad),
        extra: None::<Empty>,
    }))
}
