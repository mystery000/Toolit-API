//use crate::fault::Fault;
//use crate::util::has_role;
//use crate::models::{Claims, Rating};
//use crate::util::{DataResponse, Empty};
//use cosmos_utils::get;
//use warp::reject;
//
//pub async fn rating_get(
//    office_id: String,
//    craftsman_id: String,
//    rating_id: String,
//    claims: Claims,
//    _v: u8,
//) -> Result<impl warp::Reply, warp::Rejection> {
//    let (craftsman, _): (Craftsman, _) = get(CRAFTSMAN_COLLECTION, [&office_id], &craftsman_id).await?;
//    if has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
//        return Err(reject::custom(Fault::Forbidden(format!(
//            "",
//            rating.craftsman_id, craftsman_id
//        ))));
//    }
//    if craftsman.craftsman_id != craftsman_id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "craftsman_id does not match url ({} != {}).",
//            rating.craftsman_id, craftsman_id
//        ))));
//    }
//
//    if rating.id != rating_id {
//        return Err(reject::custom(Fault::IllegalArgument(format!(
//            "rating_id does not match url ({} != {}).",
//            rating.id, rating_id
//        ))));
//    }
//
//    Ok(warp::reply::json(&DataResponse {
//        data: Some(rating),
//        extra: None::<Empty>,
//    }))
//}
