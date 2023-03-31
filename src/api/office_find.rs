use crate::fault::Fault;
use crate::models::{Claims, Office};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::OFFICE_COLLECTION;
use cosmos_utils::query_crosspartition;
use geojson::GeoJson;
use warp::reject;

pub async fn office_find(
    r: DataRequest<GeoJson, Empty>,
    _claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let location;
    if let Some(q) = r.data {
        location = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }
    // Find the offices that have a geojson that encompasses the provided geojson
    let q = format!(
        "SELECT * FROM {} o WHERE ST_WITHIN({}, o.area)",
        OFFICE_COLLECTION,
        serde_json::to_string(&location).map_err(|_| {
            warp::reject::custom(Fault::IllegalArgument(format!(
                "Could not convert location to GeoJson string"
            )))
        })?
    );
    let offices: Vec<Office> = query_crosspartition(OFFICE_COLLECTION, [()], q, -1, true).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(offices),
        extra: None::<Empty>,
    }))
}
