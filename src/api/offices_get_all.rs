use crate::models::{Claims, Office};
use crate::util::{DataResponse, Empty};
use crate::OFFICE_COLLECTION;
use cosmos_utils::query_crosspartition;

pub async fn offices_get_all(_claims: Claims, _v: u8) -> Result<impl warp::Reply, warp::Rejection> {
    let q = format!("SELECT * FROM {}", OFFICE_COLLECTION);
    let offices: Vec<Office> = query_crosspartition(OFFICE_COLLECTION, [()], q, -1, true).await?;
    Ok(warp::reply::json(&DataResponse {
        data: Some(offices),
        extra: None::<Empty>,
    }))
}
