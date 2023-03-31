use crate::fault::Fault;
use crate::models::{Claims, CraftsmanNote, RoleFlags};
use crate::util::{has_role, DataRequest, DataResponse, Empty};
use crate::CRAFTSMAN_NOTE_COLLECTION;
use chrono::Utc;
use cosmos_utils::modify;
use warp::reject;

pub async fn craftsman_note_put(
    office_id: String,
    craftsman_id: String,
    craftsman_note_id: String,
    r: DataRequest<CraftsmanNote, Empty>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let new_note;
    if let Some(q) = r.data {
        new_note = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if office_id != new_note.office_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Office id does not match the url {} != {}",
            office_id, new_note.office_id
        ))));
    }

    if craftsman_id != new_note.craftsman_id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted craftsman id is not the same as the url {} != {}",
            craftsman_id, new_note.craftsman_id
        ))));
    }

    if craftsman_note_id != new_note.id {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Submitted note id is not the same as the url {} != {}",
            craftsman_note_id, new_note.craftsman_id
        ))));
    }

    if !has_role(Some(&office_id), &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "Caller is not a content admin for the office of the craftsman"
        ))));
    }

    let note = modify(
        CRAFTSMAN_NOTE_COLLECTION,
        [&office_id],
        &craftsman_note_id,
        |mut note: CraftsmanNote| {
            note.text = new_note.text.clone();
            note.modified = Utc::now();
            Ok(note)
        },
    )
    .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(note),
        extra: None::<Empty>,
    }))
}
