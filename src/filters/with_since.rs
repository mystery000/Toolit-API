use crate::fault::Fault;
use chrono::prelude::*;
use chrono::Duration;
use warp::{reject, Filter, Rejection};

pub fn with_since() -> impl Filter<Extract = (Option<DateTime<Utc>>,), Error = Rejection> + Clone {
    warp::header::optional::<String>("If-Range").and_then(|h: Option<String>| async move {
        if let Some(h) = h {
            // Ensure ascii.
            if !h.is_ascii() {
                return Err(reject::custom(Fault::IllegalArgument(String::from(
                    "If-Range header is not ASCII.",
                ))));
            }

            // Ensure GMT.
            if !h.ends_with(" GMT") {
                return Err(reject::custom(Fault::IllegalArgument(String::from(
                    "If-Range header is not in GMT.",
                ))));
            }

            let mut h = h.clone();

            // Strip timezone.
            h.truncate(h.len() - 4);

            match Utc.datetime_from_str(&h, "%a, %d %b %Y %H:%M:%S") {
                Ok(t) => {
                    // Add a 10 second fuzzy factor.
                    let t = t + Duration::seconds(-10);

                    Ok(Some(t))
                }
                Err(err) => Err(reject::custom(Fault::IllegalArgument(format!(
                    "Could not parse If-Range header ({}): {}.",
                    h, err
                )))),
            }
        } else {
            Ok(None)
        }
    })
}
