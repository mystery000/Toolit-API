use crate::fault::Fault;
use crate::models::{Claims, Office, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::OFFICE_COLLECTION;
use cosmos_utils::insert;
use geojson::{GeoJson, Value};
use uuid::Uuid;
use warp::reject;

pub async fn office_post(
    r: DataRequest<Office, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut office;
    if let Some(q) = r.data {
        office = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if !has_role(None, &claims, RoleFlags::GLOBAL_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    match &office.area {
        GeoJson::Geometry(geometry) => match geometry.value {
            Value::Polygon(_) => (),
            _ => {
                return Err(reject::custom(Fault::IllegalArgument(format!(
                    "Office area has to be a polygon"
                ))));
            }
        },
        _ => {
            return Err(reject::custom(Fault::IllegalArgument(format!(
                "Office area has to be a polygon"
            ))));
        }
    }

    office.id = Uuid::new_v4().to_string();
    office.modified = chrono::Utc::now();

    insert(OFFICE_COLLECTION, [&office.id], &office, None).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&office),
        extra: None::<Empty>,
    }))
}
