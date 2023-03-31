use crate::fault::Fault;
use crate::models::{Ad, Claims, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::AD_COLLECTION;
use cosmos_utils::modify;
use warp::reject;

pub async fn ad_put(
    ad_id: String,
    r: DataRequest<Ad, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_ad;
    if let Some(q) = r.data {
        new_ad = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if ad_id != new_ad.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "ad id does not match the url {} != {}",
            ad_id, new_ad.id
        ))));
    }

    if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    let ad = modify(AD_COLLECTION, [&new_ad.id], &new_ad.id, |_ad: Ad| {
        let mut ad = new_ad.clone();
        ad.modified = chrono::Utc::now();
        Ok(ad)
    })
    .await?;

    return Ok(warp::reply::json(&DataResponse {
        data: Some(&ad),
        extra: None::<Empty>,
    }));
}
