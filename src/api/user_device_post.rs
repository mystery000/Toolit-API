use crate::fault::Fault;
use crate::models::{Claims, Device, User};
use crate::push;
use crate::util::{DataRequest, DataResponse, Empty};
use crate::{NOTIFICATION_HUB_ACCOUNT, USER_COLLECTION};
use chrono::Utc;
use cosmos_utils::{get, upsert};
use warp::reject;

pub async fn user_device_post(
    user_id: String,
    r: DataRequest<Device, String>,
    claims: Claims,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let device;
    if let Some(q) = r.data {
        device = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    if user_id != claims.sub {
        return Err(reject::custom(Fault::Forbidden(format!(
            "User id does not match signed in user ({} != {}).",
            user_id, claims.sub
        ))));
    }

    let (mut user, etag): (User, String) =
        get(USER_COLLECTION, [&user_id], user_id.clone()).await?;

    // Duplicate?
    let mut found = false;
    for d in user.devices.iter_mut() {
        if d.handle == device.handle {
            if d.app_id == device.app_id
                && d.build == device.build
                && d.os == device.os
                && d.os_ver == device.os_ver
            {
                // Already registered? Return success for idempotancy.
                return Ok(warp::reply::json(&DataResponse {
                    data: Some(&user),
                    extra: None::<Empty>,
                }));
            } else {
                // Update.
                d.app_id = device.app_id.clone();
                d.build = device.build;
                d.os = device.os.clone();
                d.os_ver = device.os_ver.clone();
                found = true;
                break;
            }
        }
    }
    if !found {
        // Add device.
        user.devices.push(device);
    }

    // Update timestamp.
    user.modified = Utc::now();

    upsert(USER_COLLECTION, [&user_id], &user, Some(&etag)).await?;

    // Push notification that the registration was successful.
    match push::register_device_success(&user, None, &NOTIFICATION_HUB_ACCOUNT).await {
        Ok(()) => {}
        Err(_) => {}
    };

    Ok(warp::reply::json(&DataResponse {
        data: Some(user),
        extra: None::<Empty>,
    }))
}
