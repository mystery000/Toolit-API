use crate::fault::Fault;
use crate::models::{Claims, Craft, CraftStatus, Craftsman};
use crate::util::{DataRequest, DataResponse, Empty};
use crate::CRAFTSMAN_COLLECTION;
use cosmos_utils::CosmosSaga;
use serde::Serialize;
use uuid::Uuid;
use warp::reject;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<'a> {
    pub craftsman: &'a Craftsman,
    pub craft_id: &'a str,
}

pub async fn craft_apply(
    office_id: String,
    craftsman_id: String,
    r: DataRequest<Craft, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut craft;
    if let Some(q) = r.data {
        craft = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    // NOTE: Craftsman ID is the same as the user id
    if craftsman_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller needs to be the craftsman."
        ))));
    }

    craft.status = CraftStatus::Applied;
    craft.certificate_id = None;
    craft.id = Uuid::new_v4().to_string();

    let mut saga = CosmosSaga::new();
    let craftsman = saga
        .modify(
            CRAFTSMAN_COLLECTION,
            [&office_id],
            &craftsman_id,
            |mut craftsman: Craftsman| async {
                craftsman.crafts.push(craft.clone());
                craftsman.modified = chrono::Utc::now();
                Ok(craftsman)
            },
        )
        .await?;
    saga.finalize().await;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&Response {
            craftsman: &craftsman,
            craft_id: &craft.id,
        }),
        extra: None::<Empty>,
    }))
}
