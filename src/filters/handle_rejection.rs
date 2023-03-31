use crate::fault::{Fault, FaultCode};
use crate::util::log;
use cosmos_utils::{CosmosErrorKind, CosmosErrorStruct};
use serde::Serialize;
use std::convert::Infallible;
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Serialize)]
struct FaultResponse {
    fault: FaultResponseBody,
}

#[derive(Serialize)]
struct FaultResponseBody {
    code: i32,
    text: String,
}

// This function receives a `Rejection` and tries to return a custom
// value, otherwise simply passes the rejection along.
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let (status, code, text) = parse_error(&err);
    let json = warp::reply::json(&FaultResponse {
        fault: FaultResponseBody {
            code,
            text: text.into(),
        },
    });
    Ok(warp::reply::with_status(json, status))
}

pub fn parse_error(err: &Rejection) -> (StatusCode, i32, String) {
    let status;
    let code;
    let text;
    let g; // Helper variable for keeping owned string.

    if err.is_not_found() {
        code = FaultCode::NotFound as i32;
        status = StatusCode::NOT_FOUND;
        text = "Not found.";
    } else if let Some(fault) = err.find::<Fault>() {
        match fault {
            Fault::Unspecified(g) => {
                code = FaultCode::Unspecified as i32;
                status = StatusCode::INTERNAL_SERVER_ERROR; //StatusCode::from_u16(500).unwrap();
                text = g;
            }
            Fault::Set(_) => {
                code = FaultCode::Set as i32;
                status = StatusCode::INTERNAL_SERVER_ERROR;
                text = "Set.";
            }
            Fault::ApiLevelNoLongerSupported => {
                code = FaultCode::ApiLevelNoLongerSupported as i32;
                status = StatusCode::UPGRADE_REQUIRED;
                text = "Api level no longer supported.";
            }
            Fault::Throttling => {
                code = FaultCode::Throttling as i32;
                status = StatusCode::TOO_MANY_REQUESTS;
                text = "Throttling.";
            }
            Fault::Duplicate(g) => {
                code = FaultCode::Duplicate as i32;
                status = StatusCode::CONFLICT;
                text = g;
            }
            Fault::WrongPassword => {
                code = FaultCode::WrongPassword as i32;
                status = StatusCode::BAD_REQUEST;
                text = "Wrong password.";
            }
            Fault::NotFound(g) => {
                code = FaultCode::NotFound as i32;
                status = StatusCode::NOT_FOUND;
                text = g
            }
            Fault::Unauthorized => {
                code = FaultCode::Unauthorized as i32;
                status = StatusCode::UNAUTHORIZED;
                text = "Unauthorized.";
            }
            Fault::Forbidden(g) => {
                code = FaultCode::Forbidden as i32;
                status = StatusCode::FORBIDDEN;
                text = g;
            }
            Fault::IllegalArgument(g) => {
                code = FaultCode::IllegalArgument as i32;
                status = StatusCode::BAD_REQUEST;
                text = g;
            }
            Fault::IllegalState(g) => {
                code = FaultCode::IllegalState as i32;
                status = StatusCode::BAD_REQUEST;
                text = g;
            }
            Fault::Ineligible(g) => {
                code = FaultCode::Ineligible as i32;
                status = StatusCode::FORBIDDEN;
                text = g;
            }
            Fault::NoData => {
                code = FaultCode::NoData as i32;
                status = StatusCode::BAD_REQUEST;
                text = "No data.";
            }
            Fault::NoExtra => {
                code = FaultCode::NoExtra as i32;
                status = StatusCode::BAD_REQUEST;
                text = "No extra.";
            }
            Fault::Depleted => {
                code = FaultCode::Depleted as i32;
                status = StatusCode::BAD_REQUEST;
                text = "Depleted.";
            }
        }
    } else if let Some(x) = err.find::<CosmosErrorStruct>() {
        match &x.kind {
            CosmosErrorKind::NotFound => {
                code = FaultCode::NotFound as i32;
                status = StatusCode::NOT_FOUND;
                g = format!("Not found {:?}.", x.err);
            }
            CosmosErrorKind::BadRequest => {
                code = FaultCode::Unspecified as i32;
                status = StatusCode::INTERNAL_SERVER_ERROR;
                g = format!("Bad request {:?}.", x.err);
            }
            CosmosErrorKind::BlobError => {
                code = FaultCode::Unspecified as i32;
                status = StatusCode::INTERNAL_SERVER_ERROR;
                g = format!("Blob error {:?}.", x.err);
            }
            CosmosErrorKind::InternalError => {
                code = FaultCode::Unspecified as i32;
                status = StatusCode::INTERNAL_SERVER_ERROR;
                g = format!("Internal error {:?}.", x.err);
            }
            CosmosErrorKind::PreconditionFailed => {
                code = FaultCode::Unspecified as i32;
                status = StatusCode::INTERNAL_SERVER_ERROR;
                g = format!("Precondition failed {:?}.", x.err);
            }
            CosmosErrorKind::Conflict => {
                code = FaultCode::Duplicate as i32;
                status = StatusCode::CONFLICT;
                g = format!("Conflict {:?}.", x.err);
            }
            CosmosErrorKind::ModificationError(e) => {
                return parse_error(&e);
            }
        }
        text = &g;
    } else if let Some(x) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = FaultCode::IllegalArgument as i32;
        status = StatusCode::BAD_REQUEST;
        g = x.to_string();
        text = &g;
        log(format!("Deserialize error: {:?}.", x));
    } else if let Some(x) = err.find::<warp::reject::MethodNotAllowed>() {
        code = FaultCode::Unspecified as i32;
        status = StatusCode::METHOD_NOT_ALLOWED;
        g = format!("Method not allowed: {}.", x.to_string());
        text = &g;
        log(format!("Method not allowed: {:?}.", g));
    } else {
        code = FaultCode::Unspecified as i32;
        eprintln!("unhandled rejection: {:?}", err);
        status = StatusCode::INTERNAL_SERVER_ERROR;
        text = "Unhandled rejection.";
        log(format!("Unspecified error: {:?}.", err));
    }

    log(format!("Fault: {}, {}", code, text));
    (status, code, text.to_string())
}
