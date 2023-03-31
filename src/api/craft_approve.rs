use crate::fault::Fault;
use crate::models::{Claims, CraftStatus, Craftsman, Role, RoleFlags, User};
use crate::push::send_custom_pn;
use crate::util::{has_role, log, DataResponse, Empty};
use crate::{CRAFTSMAN_COLLECTION, NOTIFICATION_HUB_ACCOUNT, USER_COLLECTION};
use cosmos_utils::{get, CosmosSaga};
use warp::reject;

pub async fn craft_approve(
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
                // See if the craftsman has any approved crafts, in that case he is not a fresh new
                // craftsman
                let mut fresh = true;
                for craft in &craftsman.crafts {
                    if craft.status == CraftStatus::Approved {
                        fresh = false;
                        break;
                    }
                }
                let mut approved = false;
                for craft in &mut craftsman.crafts {
                    if craft.id == craft_id {
                        craft.status = CraftStatus::Approved;
                        approved = true;
                        break;
                    }
                }
                if approved {
                    if fresh {
                        craftsman.ratings = vec![];
                        craftsman.member_since = chrono::Utc::now();
                    }
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

    // Give the craftsman role to the user
    saga.modify(
        USER_COLLECTION,
        [&craftsman_id],
        &craftsman_id,
        |mut user: User| async {
            let craftsman_role = Role {
                flg: RoleFlags::CRAFTSMAN,
                sub: Some(format!("{} {}", office_id, craftsman_id.clone())),
            };
            user.roles.push(craftsman_role);
            Ok(user)
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

        let (craftsman, _): (Craftsman, _) =
            match get(CRAFTSMAN_COLLECTION, [&craftsman_id], &craftsman_id).await {
                Ok(r) => r,
                Err(_) => break,
            };

        for craft in craftsman.crafts {
            if craft_id == craft.id {
                // Send PN to the craftsman
                if let Err(e) = send_custom_pn(
                    &craftsman_user,
                    &format!(
                        "Grattis! Din ansökan som {} är accepterad",
                        craft.swedish_name()
                    ),
                    None,
                    &NOTIFICATION_HUB_ACCOUNT,
                )
                .await
                {
                    log(format!("Could not send PN in craft_approve due to {}", e));
                }
                break;
            }
        }
        break;
    }

    Ok(warp::reply::json(&DataResponse {
        data: Some(&craftsman),
        extra: None::<Empty>,
    }))
}
