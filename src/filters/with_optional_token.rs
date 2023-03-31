use crate::fault::Fault;
use crate::models::Claims;
use crate::ACCESS_TOKEN_SECRET;
use jsonwebtoken::{decode, DecodingKey, Validation}; // errors::ErrorKind
use warp::{reject, Filter, Rejection};

pub fn with_optional_token() -> impl Filter<Extract = (Option<Claims>,), Error = Rejection> + Clone
{
    warp::header::optional::<String>("Authorization").and_then(|h: Option<String>| async move {
        if let Some(h) = h {
            if h.starts_with("Bearer ") {
                let g: String = h.chars().skip(7).collect();

                // Parse.
                let validation = Validation {
                    ..Validation::default()
                };
                let token_data = match decode::<Claims>(
                    &g,
                    &DecodingKey::from_secret(ACCESS_TOKEN_SECRET.as_ref()),
                    &validation,
                ) {
                    Ok(c) => Ok(c),
                    Err(_) => Err(reject::custom(Fault::Unauthorized)),
                    // Err(err) => match *err.kind() {
                    //     ErrorKind::InvalidToken => Err(reject::custom(Fault::Unauthorized)),
                    //     ErrorKind::ExpiredSignature => Err(reject::custom(Fault::Unauthorized)),
                    //     _ => Err(reject::custom(Fault::Unauthorized)),
                    // },
                };

                match token_data {
                    Ok(t) => Ok(Some(t.claims)),
                    Err(err) => Err(err),
                }
            } else {
                Err(reject::custom(Fault::Unauthorized))
            }
        } else {
            Ok(None)
        }
    })
}
