use crate::fault::Fault;
use warp::{reject, Filter, Rejection};

pub fn with_range() -> impl Filter<Extract = (u16,), Error = Rejection> + Clone {
    warp::header::optional::<String>("Range").and_then(|h: Option<String>| async move {
        if let Some(h) = h {
            if h.starts_with("items=0-") {
                let g: String = h.chars().skip(8).collect();

                if g.len() == 0 {
                    Ok(100) // Magic number is default paging length.
                } else {
                    match g.parse::<u16>() {
                        Ok(n) => Ok(n),
                        Err(err) => Err(reject::custom(Fault::IllegalArgument(format!(
                            "Could not parse Range header ({}): {}.",
                            h, err
                        )))),
                    }
                }
            } else {
                Err(reject::custom(Fault::IllegalArgument(format!(
                    "Could not parse Range header {}.",
                    h
                ))))
            }
        } else {
            Err(reject::custom(Fault::IllegalArgument(String::from(
                "Range header is required.",
            ))))
        }
    })
}
