use crate::fault::Fault;
use crate::models::{Claims, CraftStatus, Craftsman, RoleFlags, User};
use crate::push::send_custom_pn;
use crate::util::{has_role, log, DataResponse, Empty};
use crate::{CRAFTSMAN_COLLECTION, NOTIFICATION_HUB_ACCOUNT, USER_COLLECTION};
use cosmos_utils::{get, CosmosSaga};
use warp::reject;

pub async fn craft_reject(
    office_id: String,
    craftsman_id: String,
    craft_id: String,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    if !has_role(
        Some(office_id.as_str()),
        &claims,
        RoleFlags::OFFICE_CONTENT_ADMIN,
    ) {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User does not have sufficient roles."
        ))));
    }

    let mut saga = CosmosSaga::new();
    let craftsman = saga
        .modify(
            CRAFTSMAN_COLLECTION,
            [&office_id],
            &craftsman_id,
            |mut craftsman: Craftsman| async {
                let mut rejected = false;
                for craft in &mut craftsman.crafts {
                    if craft.id == craft_id {
                        craft.status = CraftStatus::Rejected;
                        rejected = true;
                        break;
                    }
                }
                if rejected {
                    craftsman.modified = chrono::Utc::now();
                    Ok(craftsman)
                } else {
                    Err(warp::reject::custom(Fault::NotFound(format!(
                        "Could not find craft with id {} for craftsman {}",
                        craft_id, craftsman_id
                    ))))
                }
            },
        )
        .await?;
    saga.finalize().await;

    // NOTE: Loop simply so we can skip this if needed using breaks, a better option would be to have a goto
    for _ in 0..1i32 {
        let (craftsman_user, _): (User, _) =
            match get(USER_COLLECTION, [&craftsman_id], &craftsman_id).await {
                Ok(r) => r,
                Err(_) => break,
            };

        // Send PN to the craftsman
        if let Err(e) = send_custom_pn(
            &craftsman_user,
            &format!("Tyvärr nekades din ansökan som hantverkare. Vänligen kontakta support",),
            None,
            &NOTIFICATION_HUB_ACCOUNT,
        )
        .await
        {
            log(format!("Could not send PN in craft_reject due to {}", e));
        }
        break;
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(&craftsman),
        extra: None::<Empty>,
    }))
}
