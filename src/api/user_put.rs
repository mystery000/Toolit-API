use crate::fault::Fault;
use crate::models::{Claims, User};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::USER_COLLECTION;
use cosmos_utils::modify;
use warp::reject;

pub async fn user_put(
    user_id: String,
    r: DataRequest<User, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_user;
    if let Some(q) = r.data {
        new_user = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if user_id != new_user.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted user id is not the same as the url {} != {}",
            user_id, new_user.id
        ))));
    }

    if user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the PUT user"
        ))));
    }

    let user = modify(USER_COLLECTION, [&user_id], &user_id, |user: User| {
        let mut new_user = new_user.clone();
        new_user.deleted = user.deleted;
        new_user.test = user.test;
        new_user.roles = user.roles;
        new_user.devices = user.devices;
        new_user.ratings = user.ratings;
        new_user.images = user.images;
        new_user.started = user.started;
        new_user.nid = user.nid;
        new_user.email = user.email;
        new_user.office_ids = user.office_ids;
        new_user.modified = chrono::Utc::now();
        Ok(new_user)
    })
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(user),
        extra: None::<Empty>,
    }))
}
