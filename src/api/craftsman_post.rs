use crate::fault::Fault;
use crate::models::{Claims, Craftsman};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::CRAFTSMAN_COLLECTION;
use cosmos_utils::insert;
use warp::reject;

pub async fn craftsman_post(
    office_id: String,
    r: DataRequest<Craftsman, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut craftsman;
    if let Some(q) = r.data {
        craftsman = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }
    if craftsman.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            craftsman.office_id, office_id
        ))));
    }

    if craftsman.user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Calling user is not the craftsman user"
        ))));
    }

    // If the sending user is not an office admin then the craftsman is only applying and has
    // to be approved
    craftsman.id = craftsman.user_id.clone();
    craftsman.crafts = vec![];
    craftsman.ratings = vec![];
    craftsman.member_since = chrono::Utc::now();
    craftsman.modified = chrono::Utc::now();

    insert(CRAFTSMAN_COLLECTION, [&office_id], &craftsman, None).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&craftsman),
        extra: None::<Empty>,
    }))
}
