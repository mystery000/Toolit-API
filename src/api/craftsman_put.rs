use crate::fault::Fault;
use crate::models::{Claims, Craftsman, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::CRAFTSMAN_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn craftsman_put(
    office_id: String,
    craftsman_id: String,
    r: DataRequest<Craftsman, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_craftsman;
    if let Some(q) = r.data {
        new_craftsman = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if office_id != new_craftsman.office_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Office id does not match the url {} != {}",
            office_id, new_craftsman.office_id
        ))));
    }

    if craftsman_id != new_craftsman.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted craftsman id is not the same as the url {} != {}",
            craftsman_id, new_craftsman.id
        ))));
    }

    // NOTE: Craftsman id is the same as the user id
    if craftsman_id != claims.sub
        && !has_role(
            Some(&office_id),
            &claims,
            RoleFlags::OFFICE_PERSONNEL_ADMIN | RoleFlags::OFFICE_CONTENT_ADMIN,
        )
    {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the PUT craftsman"
        ))));
    }

    let craftsman = modify(
        CRAFTSMAN_COLLECTION,
        [&office_id],
        &craftsman_id,
        |craftsman: Craftsman| {
            let mut new_craftsman = new_craftsman.clone();
            new_craftsman.deleted = craftsman.deleted;
            new_craftsman.crafts = craftsman.crafts;
            new_craftsman.completed_jobs = craftsman.completed_jobs;
            new_craftsman.member_since = craftsman.member_since;
            new_craftsman.user_id = craftsman.user_id;
            new_craftsman.ratings = craftsman.ratings;
            new_craftsman.frozen = craftsman.frozen;
            new_craftsman.modified = Utc::now();
            Ok(new_craftsman)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(craftsman),
        extra: None::<Empty>,
    }))
}
