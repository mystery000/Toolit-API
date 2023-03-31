use crate::{
    fault::Fault,
    models::{Claims, RoleFlags},
    APPLICATION_INSIGHTS_TELEMETRY_CLIENT,
};
use appinsights::telemetry::SeverityLevel;
use argon2::{self, Config};
use base64::encode;
pub use orion::aead::{seal, SecretKey};
use rand::{distributions::Distribution, seq::SliceRandom, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use warp::reject;

pub fn log<S: Into<String>>(msg: S) {
    let msg = msg.into();
    dbg!(&msg);
    APPLICATION_INSIGHTS_TELEMETRY_CLIENT.track_trace(msg, SeverityLevel::Information);
}

// This is only used for serialize.
//#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn _is_zero(num: &u32) -> bool {
    *num == 0
}

// // This is only used for serialize.
// //#[allow(clippy::trivially_copy_pass_by_ref)]
// pub fn is_blank(value: &str) -> bool {
//     value.is_empty()
// }

//#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn is_false(value: &bool) -> bool {
    return !value;
}

// This is only used for serialize.
//#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn is_none<T>(option: &Option<T>) -> bool {
    match option {
        Some(_) => false,
        None => true,
    }
}

// This is only used for serialize.
//#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn is_empty<T>(value: &Vec<T>) -> bool {
    value.is_empty()
}

#[derive(Deserialize)]
pub struct DataRequest<T, U> {
    pub data: Option<T>,
    pub extra: Option<U>,
}

#[derive(Serialize)]
pub struct DataResponse<T, U> {
    #[serde(skip_serializing_if = "is_none")]
    pub data: Option<T>,

    #[serde(skip_serializing_if = "is_none")]
    pub extra: Option<U>,
}

// Helper type for no request/response.
#[derive(Serialize, Deserialize)]
pub struct Empty {}

// Generate random string of given distribution.
struct Symbols;

impl Distribution<char> for Symbols {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        *b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNOPQRSTUVWXYZ023456789"
            .choose(rng)
            .unwrap() as char
    }
}

#[allow(dead_code)]
pub fn random_string(n: usize) -> String {
    thread_rng().sample_iter(&Symbols).take(n).collect()
}

// Generate random string of given distribution.
struct Digits;

impl Distribution<char> for Digits {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        *b"0123456789".choose(rng).unwrap() as char
    }
}

//pub fn random_digit_string(n: usize) -> String {
//    thread_rng().sample_iter(&Digits).take(n).collect()
//}

// Hash string using Argon2.
pub fn hash(text: &[u8]) -> String {
    let salt = rand::thread_rng().gen::<[u8; 32]>();
    let config = Config::default();
    argon2::hash_encoded(text, &salt, &config).unwrap()
}

// Verify hash using Argon2.
pub fn verify_hash(hash: &str, password: &[u8]) -> bool {
    argon2::verify_encoded(hash, password).unwrap_or(false)
}

///// Returns the partition key and the specific id split up
//pub fn extract_partition_and_sub(subject: &str) -> Result<(&str, Option<&str>), ()> {
//    let mut iter = subject.split(" ");
//    let part = match iter.next() {
//        Some(s) => s,
//        None => return Err(()),
//    };
//
//    let spec = iter.next();
//
//    Ok((part, spec))
//}

///// A sub id is made up of either two strings or one. If the claim is regarding an area
///// then there is only one string, if the claim is regarding a unit then the area_id of the unit
///// will also be included with the unit_id appended after it with a space as a delimiter.
///// The global admin does not use a sub and as such is not considered for this.
///// The `extract_partition_and_sub` function can be used to extract these.
//pub fn create_sub_id(area_id: &str, unit_id: Option<&str>) -> String {
//    if let Some(unit_id) = unit_id {
//        format!("{} {}", area_id, unit_id)
//    } else {
//        format!("{}", area_id)
//    }
//}

///// Will return true if the claims has a role that both corresponds with the given valid flags as
///// well as points to the correct property id.
///// This function is conservative and will not return true if the valid_flags points to a type of
///// admin that is not directly handling the provided property_id.
///// That means that an AREA CONTENT ADMIN will not be authorized to use the provided unit property
///// id even if the unit exists in the correct area. Likewise a global content admin is not allowed
///// to change areas specified in property_id.
//pub fn is_authorized<S: ToString>(
//    property_id: Option<S>,
//    claims: &Claims,
//    valid_flags: RoleFlags,
//) -> bool {
//    if let Some(r) = &claims.rol {
//        for e in r.iter() {
//            // Does the current role intersect with the required roles
//            if !e.flg.intersects(valid_flags) {
//                continue;
//            }
//            if let Some(g) = &e.sub {
//                let claim_sub = match extract_partition_and_sub(g) {
//                    // If this claim regards an area then the returned value is simply an area_id
//                    Ok((area_id, None)) => area_id,
//                    // If this claim regards a unit then the returned value is simply a unit_id
//                    Ok((_, Some(unit_id))) => unit_id,
//                    Err(_) => continue,
//                };
//                if let Some(property_id) = &property_id {
//                    let property_id = property_id.to_string();
//                    if property_id == claim_sub {
//                        return true;
//                    }
//                }
//            } else {
//                // If the user is a global admin then no sub is provided and if the property_id is
//                // None
//                if let None = property_id {
//                    return true;
//                }
//            }
//        }
//    }
//    return false;
//}

/// If sub is `Some` and claims has one of the provided flags for the given subject then returns
/// true. If sub is `None` and claims has one of the provided flags then return true. Else return
/// false.
pub fn has_role(sub: Option<&str>, claims: &Claims, valid_flags: RoleFlags) -> bool {
    for e in &claims.rol {
        // Does the current role intersect with the required roles.
        if !e.flg.intersects(valid_flags) {
            continue;
        }

        if let Some(g) = sub {
            if let Some(s) = &e.sub {
                if s.starts_with(g) {
                    return true;
                }
            }
        } else {
            // No subject provided, so a match on flags suffices.
            return true;
        }
    }
    return false;
}

pub fn decrypt_string(
    encrypted_string: &str,
    password: &SecretKey,
) -> Result<String, warp::Rejection> {
    let encrypted_string = base64::decode(encrypted_string.as_bytes()).map_err(|_| {
        reject::custom(Fault::Unspecified(format!(
            "Could not convert encrypted string from base64",
        )))
    })?;
    let plain = orion::aead::open(&password, &encrypted_string).or_else(|_| {
        return Err(reject::custom(Fault::Unspecified(format!(
            "Could not encrypt string",
        ))));
    })?;
    Ok(String::from_utf8(plain)
        .map_err(|_| reject::custom(Fault::Unspecified(format!("Could not encrypt string",))))?)
}

pub fn encrypt_string(
    unencrypted_string: String,
    password: &SecretKey,
) -> Result<String, warp::Rejection> {
    Ok(encode(
        seal(&password, unencrypted_string.as_bytes()).or_else(|_| {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not encrypt",
            ))));
        })?,
    ))
}

pub fn encrypt_optional_string(
    unencrypted_option: Option<String>,
    password: &SecretKey,
) -> Result<Option<String>, warp::Rejection> {
    let s = match unencrypted_option {
        Some(mid) => Some(encode(seal(&password, mid.as_bytes()).or_else(|e| {
            return Err(reject::custom(Fault::Unspecified(format!(
                "Could not encrypt user: {}.",
                e.to_string()
            ))));
        })?)),
        None => None,
    };
    Ok(s)
}

// #[cfg(test)]
// mod util_tests {
//     use super::*;
//     use crate::models::Role;
//     use base64::decode;
//     use chrono::Utc;
//     use orion::aead::open;
//     use orion::aead::SecretKey;
//     use std::time::Instant;

//     #[test]
//     fn is_authorized_test() {
//         let office_id = "some_office_id";
//         let office_role = Role {
//             flg: RoleFlags::OFFICE_CONTENT_ADMIN,
//             sub: Some(office_id.to_string()),
//         };
//         let claims = Claims::new(
//             "my_user_id",
//             Utc::now() + chrono::Duration::minutes(20),
//             &Some(vec![office_role]),
//         );
//         assert!(is_authorized(
//             Some("some_office_id"),
//             &claims,
//             RoleFlags::OFFICE_CONTENT_ADMIN | RoleFlags::CRAFTSMAN
//         ));
//         assert!(is_authorized(
//             Some("some_office_id"),
//             &claims,
//             RoleFlags::OFFICE_CONTENT_ADMIN
//         ));
//         assert!(!is_authorized(
//             Some("some_office_id"),
//             &claims,
//             RoleFlags::CRAFTSMAN
//         ));

//         assert!(!is_authorized(
//             Some("some_unit_id"),
//             &claims,
//             RoleFlags::CRAFTSMAN | RoleFlags::OFFICE_CONTENT_ADMIN
//         ));
//         assert!(!is_authorized(
//             Some("some_unit_id"),
//             &claims,
//             RoleFlags::OFFICE_CONTENT_ADMIN
//         ));
//         assert!(!is_authorized(
//             Some("some_unit_id"),
//             &claims,
//             RoleFlags::CRAFTSMAN
//         ));
//         assert!(!is_authorized(
//             Some("some_unit_id"),
//             &claims,
//             RoleFlags::GLOBAL_CONTENT_ADMIN
//         ));

//         let unit_id = "some_craftsman_id";
//         let unit_role = Role {
//             flg: RoleFlags::CRAFTSMAN,
//             sub: Some(unit_id.to_string()),
//         };
//         let unit_claim = Claims::new(
//             "my_user_id",
//             Utc::now() + chrono::Duration::minutes(20),
//             &Some(vec![unit_role]),
//         );
//         assert!(is_authorized(
//             Some("some_craftsman_id"),
//             &unit_claim,
//             RoleFlags::OFFICE_CONTENT_ADMIN | RoleFlags::CRAFTSMAN
//         ));
//         assert!(!is_authorized(
//             Some("some_craftsman_id"),
//             &unit_claim,
//             RoleFlags::OFFICE_CONTENT_ADMIN
//         ));
//         assert!(is_authorized(
//             Some("some_craftsman_id"),
//             &unit_claim,
//             RoleFlags::CRAFTSMAN
//         ));
//         assert!(!is_authorized(
//             Some("some_office_id"),
//             &unit_claim,
//             RoleFlags::CRAFTSMAN
//         ));

//         let global_role = Role {
//             flg: RoleFlags::GLOBAL_CONTENT_ADMIN,
//             sub: None,
//         };
//         let global_claims = Claims::new(
//             "my_user_id",
//             Utc::now() + chrono::Duration::minutes(20),
//             &Some(vec![global_role]),
//         );
//         assert!(!is_authorized(
//             Some("some_craftsman_id"),
//             &global_claims,
//             RoleFlags::CRAFTSMAN | RoleFlags::OFFICE_CONTENT_ADMIN
//         ));
//         assert!(!is_authorized(
//             Some("some_craftsman_id"),
//             &global_claims,
//             RoleFlags::OFFICE_CONTENT_ADMIN
//         ));
//         assert!(!is_authorized(
//             Some("some_craftsman_id"),
//             &global_claims,
//             RoleFlags::CRAFTSMAN
//         ));
//         assert!(!is_authorized(
//             Some("some_craftsman_id"),
//             &global_claims,
//             RoleFlags::GLOBAL_CONTENT_ADMIN
//         ));
//         assert!(!is_authorized(
//             Some("some_office_id"),
//             &global_claims,
//             RoleFlags::GLOBAL_CONTENT_ADMIN
//         ));
//         assert!(is_authorized(
//             None::<String>,
//             &global_claims,
//             RoleFlags::GLOBAL_CONTENT_ADMIN
//         ));
//     }

//     #[tokio::test]
//     async fn retry_loop_test() {
//         let calls = std::cell::RefCell::new(vec![]);
//         retry_loop(8, || async {
//             calls.borrow_mut().push(Instant::now());
//             if calls.borrow().len() >= 4 {
//                 Ok(())
//             } else {
//                 Err(RetryLoopError::Transient(()))
//             }
//         })
//         .await
//         .unwrap();
//         let mut calls = calls.borrow_mut();
//         assert_eq!(calls.len(), 4);
//         let t = calls.pop().unwrap().elapsed().as_millis();
//         assert!(t == 0);
//         let t = calls.pop().unwrap().elapsed().as_millis();
//         assert!(t >= 200 && t <= 1200);
//         let t = calls.pop().unwrap().elapsed().as_millis();
//         assert!(t >= 300 && t <= 2200);
//         let t = calls.pop().unwrap().elapsed().as_millis();
//         assert!(t >= 350 && t <= 3200);
//     }

//     #[tokio::test]
//     #[ignore]
//     // Ignored since it requires quite a bit of time to retry several times
//     async fn rety_loop_failure() {
//         let calls = std::cell::RefCell::new(vec![]);
//         let result = retry_loop(8, || async {
//             calls.borrow_mut().push(Instant::now());
//             if 1 == 2 {
//                 return Ok(());
//             }
//             Err(RetryLoopError::Transient(()))
//         })
//         .await;
//         if let Ok(_) = result {
//             panic!();
//         }
//         let calls = calls.borrow();
//         assert_eq!(calls.len(), 8);
//     }

//     #[test]
//     fn personal_field_encryption_test() {
//         let first_name = String::from("Joe");
//         let key = SecretKey::default();
//         let cipher = encrypt_string(first_name.clone(), &key).unwrap();
//         assert!(first_name != cipher);
//         let cipher = decode(cipher).unwrap();
//         let cipher = String::from_utf8(open(&key, &cipher).unwrap()).unwrap();
//         assert_eq!(cipher, first_name);
//     }
// }
