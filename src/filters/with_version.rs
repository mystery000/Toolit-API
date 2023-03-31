use crate::fault::Fault;
use warp::{reject, Filter, Rejection};

pub fn with_version() -> impl Filter<Extract = (u8,), Error = Rejection> + Clone {
    warp::header::optional::<String>("Accept") //application/vnd.heimstaden.v1+json
        .and_then(|o: Option<String>| async move {
            if let Some(s) = o {
                // Parse header.
                if s == "*/*" {
                    // Default accept header.
                    Ok(0) // Use zero version.
                } else if s.starts_with("application/vnd.toolit.v") && s.ends_with("+json") {
                    let g: String = s.chars().skip(24).take(s.chars().count() - 29).collect(); // Magic number 24 is length of starts_with and 29 is length of starts_with plus ends_with.
                    match g.parse::<u8>() {
                        Ok(v) => Ok(v),
                        Err(_) => Err(reject::custom(Fault::IllegalArgument(format!(
                            "Could not parse Accept header ({}).",
                            s
                        )))),
                    }
                } else {
                    Err(reject::custom(Fault::IllegalArgument(format!(
                        "Malformed Accept header ({}).",
                        s
                    ))))
                }
            } else {
                Ok(0) // Use zero version.
            }
        })
}
