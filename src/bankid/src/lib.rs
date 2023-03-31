use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tokio::io::AsyncReadExt;
// NOTE: Reference - https://www.bankid.com/assets/bankid/rp/bankid-relying-party-guidelines-v3.5.pdf
const BASE_URL: &str = "https://appapi2.bankid.com";

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// RP must inform the user that an auth or
    /// sign order is already in progress for the
    /// user. Message RFA4 should be used.
    AlreadyInProgress,
    /// RP must not try the same request again.
    /// This is an internal error within RP's
    /// system and must not be communicated
    /// to the user as a BankID error.
    InvalidParameters,
    /// We may introduce new error codes without prior notice.
    /// RP must handle unknown error codes in their implementations.
    Unknown,
    /// RP does not have access to the service. RP must not try the same request again.
    /// This is an internal error within RP's
    /// system and must not be communicated
    /// to the user as a BankID error
    Unauthorized,
    /// An erroneously URL path was used. RP must not try the same request again.
    /// This is an internal error within RP's
    /// system and must not be communicated
    /// to the user as a BankID error.
    NotFound,
    /// It took too long time to transmit the request. RP must not automatically try again.
    /// This error may occur if the processing
    /// at RP or the communication is too
    /// slow. RP must inform the user.
    /// Message RFA5.
    RequestTimeout,
    /// Only http method POST is allowed. RP must not try the same request again.
    /// This is an internal error within RP's
    /// system and must not be communicated
    /// to the user as a BankID error.
    MethodNotAllowed,
    /// Adding a "charset" parameter after
    /// 'application/json' is not allowed since the
    /// MIME type "application/json" has neither
    /// optional nor required parameters.
    /// The type is missing or erroneously.
    /// RP must not try the same request again.
    /// This is an internal error within RP's
    /// system and must not be communicated
    /// to the user as a BankID error.
    UnsupportedMediaType,
    /// Internal technical error in the BankID
    /// system.
    /// RP must not automatically try again.
    /// RP must inform the user. Message
    /// RFA5.
    InternalError,
    /// The service is temporarily out of service. RP may try again without informing the
    /// user. If this error is returned repeatedly,
    /// RP must inform the user. Message
    /// RFA5.
    Maintenance,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorCode::AlreadyInProgress => {
                formatter.write_str("AlreadyInProgress")?;
            },
            ErrorCode::InvalidParameters => {
                formatter.write_str("InvalidParameters")?;
            },
            ErrorCode::Unknown => {
                formatter.write_str("Unknown")?;
            },
            ErrorCode::Unauthorized => {
                formatter.write_str("Unauthorized")?;
            },
            ErrorCode::NotFound => {
                formatter.write_str("NotFound")?;
            },
            ErrorCode::RequestTimeout => {
                formatter.write_str("RequestTimeout")?;
            },
            ErrorCode::MethodNotAllowed => {
                formatter.write_str("MethodNotAllowed")?;
            },
            ErrorCode::UnsupportedMediaType => {
                formatter.write_str("UnsupportedMediaType")?;
            },
            ErrorCode::InternalError => {
                formatter.write_str("InternalError")?;
            },
            ErrorCode::Maintenance => {
                formatter.write_str("Maintenance")?;
            },
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub error_code: ErrorCode,
    pub details: String,
}

pub enum BankIdError {
    FileSystemError(String),
    NetworkError(String),
    CertificateError(String),
    ApiError(ApiError),
    SerializationError(String),
    Failed(String),
    Unspecified(String),
    ProtocolError(String),
}

type Result<T> = std::result::Result<T, BankIdError>;

pub struct BankIdClient {
    net_client: reqwest::Client,
}

impl BankIdClient {
    async fn send<'a, T: DeserializeOwned>(&self, url: &str, body: impl Serialize) -> Result<T> {
        let resp = match self.net_client.post(url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                return Err(BankIdError::NetworkError(format!(
                    "Could not send message due to {:?}",
                    e
                )))
            }
        };
        if resp.status() != 200 {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_else(|_| String::from("Could not retrieve body text"));
            let api_error: ApiError = serde_json::from_str(&text).unwrap_or_else(|_| ApiError {
                error_code: ErrorCode::Unknown,
                details: format!("Unknown error with status code: {}", status),
            });
            return Err(BankIdError::ApiError(api_error));
        }
        let body: T = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                return Err(BankIdError::SerializationError(format!(
                    "Could not deserialize response due to {:?}",
                    e
                )))
            }
        };
        Ok(body)
    }

    // TODO: Need to verify that the response has the correct signature
    /// Create a new BankIdClient using the identity path and password and the trusted certificate
    /// path
    pub async fn new<P: AsRef<std::path::Path>>(
        ident_path: P,
        ident_pass: &str,
        trust_cert_path: P,
    ) -> Result<BankIdClient> {
        // Read the identity file
        let mut ident_buf = Vec::new();
        let mut f = match tokio::fs::File::open(ident_path).await {
            Ok(f) => f,
            Err(e) => {
                return Err(BankIdError::FileSystemError(format!(
                    "Could not open identity file due to {:?}",
                    e
                )));
            }
        };
        match f.read_to_end(&mut ident_buf).await {
            Ok(_) => (),
            Err(e) => {
                return Err(BankIdError::FileSystemError(format!(
                    "Could not read identity file due to {:?}",
                    e
                )));
            }
        };

        // Create identity
        let identity = match reqwest::Identity::from_pkcs12_der(&ident_buf, ident_pass) {
            Ok(r) => r,
            Err(e) => {
                return Err(BankIdError::CertificateError(format!(
                    "Could not convert identity file due to {:?}",
                    e
                )))
            }
        };

        // Read the certificate file
        let mut cert_buf = Vec::new();
        let mut f = match tokio::fs::File::open(trust_cert_path).await {
            Ok(f) => f,
            Err(e) => {
                return Err(BankIdError::FileSystemError(format!(
                    "Could not open certificate file due to {:?}",
                    e
                )));
            }
        };
        match f.read_to_end(&mut cert_buf).await {
            Ok(_) => (),
            Err(e) => {
                return Err(BankIdError::FileSystemError(format!(
                    "Could not read certificate file due to {:?}",
                    e
                )));
            }
        };

        // Create the certificate
        let cert = match reqwest::Certificate::from_pem(&cert_buf) {
            Ok(r) => r,
            Err(e) => {
                return Err(BankIdError::CertificateError(format!(
                    "Could not convert certificate file due to {:?}",
                    e
                )))
            }
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let net_client = match reqwest::ClientBuilder::new()
            .default_headers(headers)
            .https_only(true)
            .identity(identity)
            .add_root_certificate(cert)
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                return Err(BankIdError::Unspecified(format!(
                    "Could not create reqwest client due to {:?}",
                    e
                )))
            }
        };

        let c = BankIdClient { net_client };
        Ok(c)
    }

    /// Creates an authentication request to the given IP address and returns an OrderRef
    pub async fn auth(&self, request: AuthRequest) -> Result<AuthResp> {
        // Make HTTP auth request
        let url = format!("{}/rp/v5.1/auth", BASE_URL);
        let resp: AuthResp = self.send(&url, &request).await?;
        Ok(resp)
    }

    /// Creates a sign request to the given IP address and returns an OrderRef
    pub async fn sign(&self, request: SignRequest) -> Result<SignResp> {
        // Make HTTP auth request
        let url = format!("{}/rp/v5.1/sign", BASE_URL);
        let resp: SignResp = self.send(&url, &request).await?;
        Ok(resp)
    }

    /// Collects status of an order
    pub async fn collect(&self, order_ref: String) -> Result<CollectResp> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CollectReq<'a> {
            order_ref: &'a String,
        }

        let body = CollectReq {
            order_ref: &order_ref,
        };
        // Make HTTP collect request
        let url = format!("{}/rp/v5.1/collect", BASE_URL);
        let body: PrivCollectResp = self.send(&url, &body).await?;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PrivCollectResp {
            #[allow(dead_code)]
            order_ref: String,
            status: String,
            hint_code: Option<String>,
            completion_data: Option<CompletionData>,
        }

        let status = match body.status.as_ref() {
            "pending" => Status::Pending,
            "failed" => Status::Failed,
            "complete" => {
                let cd = match body.completion_data {
                    Some(r) => r,
                    None => {
                        return Err(BankIdError::SerializationError(format!(
                            "Missing completion data in response"
                        )))
                    }
                };
                Status::Complete(cd)
            }
            _ => {
                return Err(BankIdError::ProtocolError(format!(
                    "Unknown status: {}",
                    body.status
                )));
            }
        };

        let hint_code = match body.hint_code {
            Some(hc) => {
                // If we have a hint-code and the status is complete then something is wrong and we
                // need to return an error
                if let Status::Complete(_) = status {
                    return Err(BankIdError::ProtocolError(format!(
                        "A hint code was found for a \"complete\" status. Hintcode: {}",
                        hc
                    )));
                }

                let hc = match hc.as_str() {
                    "outstandingTransaction" => HintCode::OutstandingTransaction,
                    "noClient" => HintCode::NoClient,
                    "started" => HintCode::Started,
                    "userSign" => HintCode::UserSign,
                    "expiredTransaction" => HintCode::ExpiredTransaction,
                    "certificateErr" => HintCode::CertificateErr,
                    "userCancel" => HintCode::UserCancel,
                    "cancelled" => HintCode::Cancelled,
                    _ => HintCode::Unknown,
                };
                Some(hc)
            }
            None => {
                // If we are lacking a hint-code and the status is not complete then something is
                // wrong and we need to return an error
                if let Status::Complete(_) = &status {
                    None
                } else {
                    return Err(BankIdError::ProtocolError(format!(
                        "No hint code was found for non-complete status"
                    )));
                }
            }
        };

        Ok(CollectResp {
            order_ref,
            hint_code,
            status,
        })
    }

    /// Cancel the given order_ref
    pub async fn cancel(&self, order_ref: String) -> Result<()> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CancelReq<'a> {
            order_ref: &'a String,
        }
        let body = CancelReq {
            order_ref: &order_ref,
        };

        // Make HTTP cancel request
        let url = format!("{}/rp/v5.1/cancel", BASE_URL);
        let _resp: () = self.send(&url, &body).await?;
        Ok(())
    }
}

/// class1" - (default). The transaction must be performed using a
///     card reader where the PIN code is entered on the computers
///     keyboard, or a card reader of higher class.
/// "class2" - The transaction must be performed using a card
///     reader where the PIN code is entered on the reader, or a reader
///     of higher class.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CardReader {
    Class1,
    Class2,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    /// class1" - (default). The transaction must be performed using a
    ///     card reader where the PIN code is entered on the computers
    ///     keyboard, or a card reader of higher class.
    /// "class2" - The transaction must be performed using a card
    ///     reader where the PIN code is entered on the reader, or a reader
    ///     of higher class.
    /// <no value> - defaults to "class1".
    ///     This condition should be combined with a
    ///     certificatePolicies for a smart card to avoid undefined
    ///     behavior.
    #[serde(skip_serializing_if = "is_none")]
    pub card_reader: Option<CardReader>,
    /// The oid in certificate policies in the user certificate. List of
    /// String. One wildcard ”*” is allowed from position 5 and
    /// forward ie. 1.2.752.78.*
    /// The values for production BankIDs are:
    /// "1.2.752.78.1.1" - BankID on file
    /// "1.2.752.78.1.2" - BankID on smart card
    /// "1.2.752.78.1.5" - Mobile BankID
    /// "1.2.752.71.1.3" - Nordea e-id on file and on smart card.
    /// The values for test BankIDs are:
    /// "1.2.3.4.5" - BankID on file
    /// "1.2.3.4.10" - BankID on smart card
    /// "1.2.3.4.25" - Mobile BankID
    /// "1.2.752.71.1.3" - Nordea e-id on file and on smart card.
    /// “1.2.752.60.1.6” - Test BankID for some BankID Banks
    #[serde(skip_serializing_if = "is_none")]
    pub certificate_policies: Option<Vec<String>>,
    /// The cn (common name) of the issuer. List of String.
    /// Wildcards are not allowed. Nordea values for production:
    /// "Nordea CA for Smartcard users 12" - E-id on smart card
    /// issued by Nordea CA.
    /// "Nordea CA for Softcert users 13" - E-id on file issued by
    /// Nordea CA
    /// Example Nordea values for test:
    /// "Nordea Test CA for Smartcard users 12" - E-id on smart card
    /// issued by Nordea CA.
    /// "Nordea Test CA for Softcert users 13" - E-id on file issued by
    /// Nordea CA
    #[serde(skip_serializing_if = "is_none")]
    pub issuer_cn: Option<Vec<String>>,
    /// Users of iOS and Android devices may use fingerprint for
    /// authentication and signing if the device supports it and the
    /// user configured the device to use it. Boolean. No other devices
    /// are supported at this point.
    /// If set to true, the users are allowed to use fingerprint.
    /// If set to false, the users are not allowed to use fingerprint.
    #[serde(skip_serializing_if = "is_none")]
    pub allow_fingerprint: Option<bool>,
    /// The tokenStartRequired replaces the autostartTokenRequired.
    /// Boolean. If present, and set to true, one of the following
    /// methods must be used to start the client:
    /// • According to chapter 4.2 in this document (animated
    /// QR).
    /// • According to chapter 3 in this document
    /// (autoStartToken in an URL).
    /// • According to chapter 4.1in this document
    /// (autoStartToken in a static QR).
    #[serde(skip_serializing_if = "is_none")]
    pub token_start_required: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResp {
    pub order_ref: String,
    pub auto_start_token: String,
    pub qr_start_token: String,
    pub qr_start_secret: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignResp {
    pub order_ref: String,
    pub auto_start_token: String,
    pub qr_start_token: String,
    pub qr_start_secret: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HintCode {
    OutstandingTransaction,
    NoClient,
    Started,
    UserSign,
    ExpiredTransaction,
    CertificateErr,
    UserCancel,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Status {
    Pending,
    Failed,
    Complete(CompletionData),
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub personal_number: String,
    pub name: String,
    pub given_name: String,
    pub surname: String,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub ip_address: String,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cert {
    pub not_before: String,
    pub not_after: String,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionData {
    pub user: User,
    pub device: Device,
    pub cert: Cert,
    pub signature: String,
    pub ocsp_response: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CollectResp {
    pub status: Status,
    pub hint_code: Option<HintCode>,
    pub order_ref: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Formatting {
    SimpleMarkdownV1,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignRequest {
    pub user_ip: std::net::IpAddr,
    #[serde(skip_serializing_if = "is_none")]
    pub personal_number: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub requirement: Option<Requirement>,
    pub user_visible_data: String,
    #[serde(skip_serializing_if = "is_none")]
    pub user_non_visible_data: Option<String>,
    #[serde(skip_serializing_if = "is_none")]
    pub user_visible_data_format: Option<Formatting>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRequest {
    #[serde(skip_serializing_if = "is_none")]
    pub personal_number: Option<String>,
    pub end_user_ip: IpAddr,
    #[serde(skip_serializing_if = "is_none")]
    pub requirement: Option<Requirement>,
}

// Used to skip serializing fields if they are none
fn is_none<T>(opt: &Option<T>) -> bool {
    opt.is_none()
}
