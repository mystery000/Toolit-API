use crate::fault::Fault;
use crate::models::{Claims, Craftsman};
use crate::util::{DataResponse, Empty};
use crate::CRAFTSMAN_COLLECTION;
use cosmos_utils::get;
use warp::reject;

pub async fn craftsman_get(
    office_id: String,
    craftsman_id: String,
    _claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (craftsman, _etag): (Craftsman, _) =
        get(CRAFTSMAN_COLLECTION, [&office_id], &craftsman_id).await?;
    if craftsman.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            craftsman.office_id, office_id
        ))));
    }

    if craftsman.id != craftsman_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "craftsman_id does not match url ({} != {}).",
            craftsman.id, craftsman_id
        ))));
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(craftsman),
        extra: None::<Empty>,
    }))
}
