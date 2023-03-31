use crate::fault::Fault;
use crate::models::{Claims, CraftsmanNote, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::CRAFTSMAN_NOTE_COLLECTION;
use cosmos_utils::insert;
use uuid::Uuid;
use warp::reject;

pub async fn craftsman_note_post(
    office_id: String,
    craftsman_id: String,
    r: DataRequest<CraftsmanNote, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut craftsman_note;
    if let Some(q) = r.data {
        craftsman_note = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if craftsman_note.office_id != office_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "office_id does not match url ({} != {}).",
            craftsman_note.office_id, office_id
        ))));
    }

    if craftsman_note.craftsman_id != craftsman_id {
        return Err(reject::custom(Fault::IllegalArgument(format!(
            "craftsman_id does not match url ({} != {}).",
            craftsman_note.craftsman_id, craftsman_id
        ))));
    }

    if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not a content admin for the office of the craftsman"
        ))));
    }

    if craftsman_note.id == "" {
        craftsman_note.id = Uuid::new_v4().to_string();
    }
    craftsman_note.modified = chrono::Utc::now();

    insert(
        CRAFTSMAN_NOTE_COLLECTION,
        [&office_id],
        &craftsman_note,
        None,
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(&craftsman_note),
        extra: None::<Empty>,
    }))
}
