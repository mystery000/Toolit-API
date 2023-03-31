use crate::fault::Fault;
use crate::models::{AuthEmail, User};
use crate::util::{self, DataRequest, DataResponse, Empty};
use crate::SENDGRID_API_KEY;
use crate::{AUTH_EMAIL_COLLECTION, USER_COLLECTION};
use cosmos_utils::get;
use cosmos_utils::modify;
use sendgrid::v3::*;
use warp::reject;

pub async fn forgot_password(
    r: DataRequest<String, Empty>,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let email;
    if let Some(q) = r.data {
        // Normalise email.
        email = q.to_lowercase();
    } else {
        return Err(reject::custom(Fault::NoData));
    }

    // Generate new password.
    let password = util::random_string(20);

    let auth_email: AuthEmail = modify(
        AUTH_EMAIL_COLLECTION,
        [&email],
        &email,
        |mut auth_email: AuthEmail| {
            if let Some(_passhash) = &auth_email.passhash {
                // Change password.
                auth_email.passhash = Some(util::hash(password.as_bytes()));
                Ok(auth_email)
            } else {
                Err(reject::custom(Fault::Forbidden(format!(
                    "Not allowed to recover passwords if bankid is signup means"
                ))))
            }
        },
    )
    .await?;

    let (user, _etag): (User, _) =
        get(USER_COLLECTION, [&auth_email.user_id], &auth_email.user_id).await?;
    // Send email.
    let mut map = SGMap::new();
    map.insert(String::from("{{ newPassword }}"), password);

    let p = Personalization::new(Email::new("support@toolitapp.com"))
        .add_to(Email::new(&user.email))
        .add_dynamic_template_data(map);

    //FIXME: UPDATE the template ID that exists on sendgrid. Find via azure
    let m = Message::new(Email::new("support@toolitapp.com"))
        .set_template_id("d-2077b76f5b404577a20513f1596ca7bc")
        .add_personalization(p);
    let sender = Sender::new(SENDGRID_API_KEY.to_string());
    match sender.send(&m).await {
        _ => {}
    };

    Ok(warp::reply::json(&DataResponse {
        data: None::<Empty>,
        extra: None::<Empty>,
    }))
}
