use crate::fault::Fault;
use crate::models::{Claims, Office};
use crate::util::{DataResponse, Empty};
use crate::OFFICE_COLLECTION;
use cosmos_utils::get;
use warp::reject;

pub async fn office_get(
    office_id: String,
    _claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (office, _etag): (Office, _) = get(OFFICE_COLLECTION, [&office_id], &office_id).await?;

    if office.id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            office.id, office_id
        ))));
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(office),
        extra: None::<Empty>,
    }))
}
