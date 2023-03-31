use crate::fault::Fault;
use crate::models::{Claims, User};
use crate::util::{DataResponse, Empty};
use crate::USER_COLLECTION;
use cosmos_utils::get;
use warp::reject;

pub async fn user_get(
    user_id: String,
    _claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (user, _etag): (User, _) = get(USER_COLLECTION, [&user_id], &user_id).await?;

    if user.id != user_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "user_id does not match url ({} != {}).",
            user.id, user_id
        ))));
    }

    //unimplemented(); //On delete the OFFICE_CONTENT_ADMIN should be any type of office content admin not for a specific office

    Ok(warp::reply::json(&DataResponse {
        data: Some(user),
        extra: None::<Empty>,
    }))
}
