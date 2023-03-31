#![allow(dead_code)]

use warp::reject;

#[derive(Debug)]
pub enum Fault {
    Unspecified(String),
    Set(Vec<Fault>),
    ApiLevelNoLongerSupported,
    Throttling,
    Duplicate(String),
    WrongPassword,
    NotFound(String),
    Unauthorized,
    Forbidden(String),
    IllegalArgument(String),
    IllegalState(String),
    Ineligible(String),
    NoData,
    NoExtra,
    Depleted,
}

impl reject::Reject for Fault {}

use swish::SwishError;

impl From<SwishError> for Fault {
    fn from(e: SwishError) -> Self {
        match e {
            SwishError::FileSystemError(e) => {
                Fault::Unspecified(format!("Swish filesystem error: {}", e))
            }
            SwishError::NetworkError(e) => {
                Fault::Unspecified(format!("Swish network error: {}", e))
            }
            SwishError::ApiError(api_err) => Fault::Unspecified(format!(
                "Swish api error: code - {}, errors - {:?}",
                api_err.code, api_err.errors
            )),
            SwishError::SerializationError(e) => {
                Fault::Unspecified(format!("Swish serialization error: {}", e))
            }
            SwishError::VersionError(e) => {
                Fault::Unspecified(format!("Swish version error: {}", e))
            }
            SwishError::CertificateError(e) => {
                Fault::Unspecified(format!("Swish certificate error: {}", e))
            }
            SwishError::SwishHttpError(e) => Fault::Unspecified(format!("Swish http error: {}", e)),
            SwishError::Unspecified(e_str) => Fault::Unspecified(format!("Swish: {}", e_str)),
        }
    }
}

use bankid::BankIdError;

impl From<BankIdError> for Fault {
    fn from(e: BankIdError) -> Self {
        match e {
            BankIdError::FileSystemError(e) => {
                Fault::Unspecified(format!("Bankid filesystem error: {}", e))
            }
            BankIdError::CertificateError(e) => {
                Fault::Unspecified(format!("Bank id certificate error: {}", e))
            }
            BankIdError::NetworkError(e) => {
                Fault::Unspecified(format!("Bank id network error: {}", e))
            }
            BankIdError::ApiError(api_err) => Fault::Unspecified(format!(
                "Bank id api error: code - {}, details - {}",
                api_err.error_code, api_err.details
            )),
            BankIdError::SerializationError(e) => {
                Fault::Unspecified(format!("Bank id serialization error: {}", e))
            }
            BankIdError::ProtocolError(e) => {
                Fault::Unspecified(format!("Bank id protocol error: {}", e))
            }
            BankIdError::Failed(e) => {
                Fault::Unspecified(format!("Bank id returned failure: {}", e))
            }
            BankIdError::Unspecified(e_str) => Fault::Unspecified(format!("Bank id: {}", e_str)),
        }
    }
}

pub enum FaultCode {
    Unspecified = 0,
    Set = 1,
    ApiLevelNoLongerSupported = 2,
    Throttling = 3,
    Duplicate = 4,
    WrongPassword = 5,
    NotFound = 6,
    Unauthorized = 7,
    Forbidden = 8,
    IllegalArgument = 9,
    IllegalState = 10,
    Ineligible = 11,
    NoData = 12,
    NoExtra = 13,
    Depleted = 14,
}
