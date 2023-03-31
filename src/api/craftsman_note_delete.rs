use crate::fault::Fault;
use crate::models::{Claims, CraftsmanNote, RoleFlags};
use crate::util::{has_role, DataResponse, Empty};
use crate::CRAFTSMAN_NOTE_COLLECTION;
use chrono::Utc;
use cosmos_utils::CosmosSaga;
use warp::reject;

pub async fn craftsman_note_delete(
    office_id: String,
    craftsman_id: String,
    note_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut saga = CosmosSaga::new();
    let deleted_note = saga
        .modify(
            CRAFTSMAN_NOTE_COLLECTION,
            [&office_id],
            &note_id,
            |mut note: CraftsmanNote| async {
                if craftsman_id != note.craftsman_id {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "Submitted craftsman id is not the same as the url {} != {}",
                        craftsman_id, note.craftsman_id
                    ))));
                }

                if !has_role(None, &claims, RoleFlags::OFFICE_CONTENT_ADMIN) {
                    return Err(reject::custom(Fault::Forbidden(format!(
                        "User does not have sufficient roles."
                    ))));
                }
                note.deleted = true;
                note.modified = Utc::now();
                Ok(note)
            },
        )
        .await?;

    Ok(warp::reply::json(&DataResponse {
        data: Some(deleted_note),
        extra: None::<Empty>,
    }))
}
