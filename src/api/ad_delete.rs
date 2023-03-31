use crate::fault::Fault;
use crate::models::{Ad, Claims, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::AD_COLLECTION;
use cosmos_utils::modify;
use warp::reject;

pub async fn ad_delete(
    ad_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    let ad = modify(AD_COLLECTION, [&ad_id], &ad_id, |mut ad: Ad| {
        if ad_id != ad.id {
            return Err(reject::custom(Fault::Forbidden(format!(
                "ad id does not match the url {} != {}",
                ad_id, ad.id
            ))));
        }

        ad.deleted = true;
        ad.modified = chrono::Utc::now();
        Ok(ad)
    })
    .await?;

    return Ok(warp::reply::json(&DataResponse {
        data: Some(&ad),
        extra: None::<Empty>,
    }));
}
