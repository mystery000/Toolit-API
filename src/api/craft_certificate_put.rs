use crate::fault::Fault;
use crate::models::{Claims, Craftsman};
use crate::util::{DataResponse, Empty};
use crate::{CERTIFICATE_STORAGE_CONTAINER, CRAFTSMAN_COLLECTION};
use chrono::Utc;
use cosmos_utils::{get, upload_blob, upsert};
use warp::filters::multipart::FormData;
use warp::reject;

pub async fn craft_certificate_put(
    office_id: String,
    craftsman_id: String,
    craft_id: String,
    claims: Claims,
    _v: u8,
    f: FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    let (mut craftsman, etag): (Craftsman, _) =
        get(CRAFTSMAN_COLLECTION, [&office_id], &craftsman_id).await?;
    // NOTE: Craftsman id is the same as the user id
    if craftsman_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not the craftsman"
        ))));
    }
    let mut found = false;
    for craft in &mut craftsman.crafts {
        if craft.id == craft_id {
            found = true;
            let certificate_id = upload_blob(
                f,
                // TODO: Currently the app is sending these as images
                "image",
                // An empty string allows any content type
                "",
                &*CERTIFICATE_STORAGE_CONTAINER,
            )
            .await?;
            craft.certificate_id = Some(certificate_id);
            craftsman.modified = Utc::now();
            break;
        }
    }
    if !found {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Could not find craft {} for craftsman {}",
            craft_id, craftsman_id
        ))));
    }

    upsert(CRAFTSMAN_COLLECTION, [&office_id], &craftsman, Some(&etag)).await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(craftsman),
        extra: None::<Empty>,
    }))
}
