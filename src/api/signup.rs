use crate::fault::Fault;
use crate::models::{AuthEmail, AuthNid, Claims, Office, User};
use crate::util::{self, log, DataRequest, DataResponse, Empty, SecretKey};
use crate::{
    ACCESS_TOKEN_SECRET, AUTH_EMAIL_COLLECTION, AUTH_NID_COLLECTION, BANKID_NID_SECRET,
    OFFICE_COLLECTION, PRODUCTION_ENVIRONMENT, REFRESH_TOKEN_SECRET, SENDGRID_API_KEY,
    USER_COLLECTION,
};
use chrono::{prelude::*, Duration};
use cosmos_utils::{query_crosspartition, CosmosSaga};
use geojson::Geometry;
use jsonwebtoken::{encode, EncodingKey, Header};
use sendgrid::v3::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::reject;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignupType {
    Password(String),
    OpaqueNid(String),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupExtra {
    r#type: SignupType,
    location: Option<Geometry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response<'a> {
    pub access_token: &'a str,
    pub refresh_token: &'a str,
    pub user_id: &'a str,
}

pub async fn signup(
    r: DataRequest<User, SignupExtra>,
    _v: u8,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut user;
    if let Some(q) = r.data {
        user = q;
    } else {
        return Err(reject::custom(Fault::NoData));
    }
    let signuptype;
    let location;
    if let Some(q) = r.extra {
        signuptype = q.r#type;
        location = q.location;
    } else {
        return Err(reject::custom(Fault::NoExtra));
    }

    if user.id == "" {
        user.id = Uuid::new_v4().to_string();
    }

    // If no preferred name is given then the preferred name is set to the first name
    if let None = user.preferred_name {
        user.preferred_name = Some(user.first_name.clone());
    }

    // // Make date from IANA location.
    // let tz: Tz = "Europe/Stockholm".parse().unwrap(); // "Antarctica/South_Pole"
    // let dt = tz.ymd(1977, 10, 26).and_hms(7, 0, 0);
    // let fo = dt.with_timezone(&dt.offset().fix());
    // println!("tz = {}", dt);
    // user.dob = Some(fo);

    // Automatically make user a test user if on the test server.
    if !*PRODUCTION_ENVIRONMENT {
        user.test = true;
    }
    user.started = chrono::Utc::now();
    // Set modified
    user.modified = chrono::Utc::now();

    if let Some(location) = location {
        // Find the offices that have a geojson that encompasses the provided geojson
        // This can also be done with the "office_find" endpoint
        let q = format!(
            "SELECT * FROM {} o WHERE ST_WITHIN({}, o.area)",
            OFFICE_COLLECTION,
            serde_json::to_string(&location).map_err(|_| {
                warp::reject::custom(Fault::IllegalArgument(format!(
                    "Could not convert location to GeoJson string"
                )))
            })?
        );
        let offices: Vec<Office> =
            query_crosspartition(OFFICE_COLLECTION, [()], q, -1, true).await?;
        for office in offices {
            user.office_ids.push(office.id);
        }
    } else {
        let q = format!(
            "SELECT * FROM {}",
            OFFICE_COLLECTION,
        );
        // Get the first office in the list if no location was provided
        let mut offices: Vec<Office> =
            query_crosspartition(OFFICE_COLLECTION, [()], q, -1, true).await?;
        if offices.len() >= 1 {
            let office = offices.remove(0);
            user.office_ids.push(office.id);
        }
    }

    // Depending on how we signed up we either create an auth-nid or an auth-email
    let (nid, passhash) = match signuptype {
        SignupType::OpaqueNid(opaque_nid) => {
            let code = match SecretKey::from_slice(BANKID_NID_SECRET.as_bytes()) {
                Ok(r) => r,
                Err(_) => {
                    return Err(reject::custom(Fault::Unspecified(String::from(
                        "Internal server error, could not generate nid secret",
                    ))))
                }
            };
            let nid = util::decrypt_string(&opaque_nid, &code)?;
            // Set the correct nid
            user.nid = nid;
            (&user.nid, None)
        }
        SignupType::Password(password) => {
            let passhash = Some(util::hash(password.as_bytes())); // Calculate pass hash.
            (&user.nid, passhash)
        }
    };

    let mut user_signup_saga = CosmosSaga::new();
    user_signup_saga
        .insert(USER_COLLECTION, [&user.id], &user, &user.id, None)
        .await?;

    let auth_nid = AuthNid {
        id: nid.clone(),
        user_id: user.id.clone(),
    };

    user_signup_saga
        .insert(
            AUTH_NID_COLLECTION,
            [&auth_nid.id],
            &auth_nid,
            &auth_nid.id,
            None,
        )
        .await?;

    // Normalise email.
    let email = user.email.clone().to_lowercase();

    // Add email auth.
    let email_auth = AuthEmail {
        id: email,
        passhash,
        user_id: user.id.clone(),
    };

    //TODO(Jonathan): Should we try to make sure the email is a real email?
    user_signup_saga
        .insert(
            AUTH_EMAIL_COLLECTION,
            [&email_auth.id],
            &email_auth,
            &email_auth.id,
            None,
        )
        .await?;
    user_signup_saga.finalize().await;

    // Send welcome email.
    let mut map = SGMap::new();
    map.insert(String::from("firstName"), user.first_name.clone());

    let p = Personalization::new(Email::new("support@toolitapp.com"))
        .add_to(Email::new(&user.email))
        .add_dynamic_template_data(map);

    let m = Message::new(Email::new("support@toolitapp.com"))
        .set_template_id("d-a3e79ce724654cd79d894597202456f9")
        .add_personalization(p);
    let sender = Sender::new(SENDGRID_API_KEY.to_string());
    match sender.send(&m).await {
        Ok(_) => (),
        Err(e) => {
            log(format!("Could not send a welcome email due to {:?}", e));
        }
    };

    let iat = Utc::now();
    let exp = iat + Duration::minutes(20);
    let claims = Claims::new(&user.id, exp, &vec![]);

    let access_token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(ACCESS_TOKEN_SECRET.as_ref()),
    ) {
        Ok(token) => token,
        Err(error) => {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not encode access token: {}.",
                error.to_string()
            ))));
        }
    };

    let refresh_token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(REFRESH_TOKEN_SECRET.as_ref()),
    ) {
        Ok(token) => token,
        Err(error) => {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not encode refresh token: {}.",
                error.to_string()
            ))));
        }
    };

    Ok(warp::reply::json(&DataResponse {
        data: Some(&Response {
            access_token: &access_token,
            refresh_token: &refresh_token,
            user_id: &user.id,
        }),
        extra: None::<Empty>,
    }))
}
