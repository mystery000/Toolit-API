use chrono::{DateTime, Utc};
use openssl::hash::MessageDigest;
use openssl::sign::Signer;
use openssl::{
    pkcs12::{ParsedPkcs12, Pkcs12},
    sha::sha512,
};
use rust_decimal::Decimal;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

pub type Result<T> = std::result::Result<T, SwishError>;

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorObject {
    pub error_code: String,
    pub error_message: String,
    #[serde(default)]
    pub additional_information: Option<String>,
}

impl std::fmt::Display for ApiErrorObject {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(additional_information) = &self.additional_information {
            fmt.write_fmt(format_args!(
                "ApiErrorObject: error_code - {}, error_message - {}, additional_information - {}",
                self.error_code, self.error_message, additional_information
            ))?;
        } else {
            fmt.write_fmt(format_args!(
                    "ApiErrorObject: error_code - {}, error_message - {}, additional_information - none",
                    self.error_code, self.error_message))?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: u16,
    pub errors: Vec<ApiErrorObject>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_fmt(format_args!(
            "ApiError: code - {}, errors: {:?}",
            self.code, self.errors
        ))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SwishError {
    FileSystemError(String),
    Unspecified(String),
    NetworkError(String),
    SerializationError(String),
    ApiError(ApiError),
    VersionError(String),
    CertificateError(String),
    SwishHttpError(String),
}

impl std::fmt::Display for SwishError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SwishError::FileSystemError(e) => {
                fmt.write_fmt(format_args!("FileSystemError - {}", e))?;
            }
            SwishError::Unspecified(e) => {
                fmt.write_fmt(format_args!("Unspecified - {}", e))?;
            }
            SwishError::NetworkError(e) => {
                fmt.write_fmt(format_args!("NetworkError - {}", e))?;
            }
            SwishError::SerializationError(e) => {
                fmt.write_fmt(format_args!("SerializationError - {}", e))?;
            }
            SwishError::ApiError(e) => {
                fmt.write_fmt(format_args!("{}", e))?;
            }
            SwishError::VersionError(e) => {
                fmt.write_fmt(format_args!("VersionError - {}", e))?;
            }
            SwishError::CertificateError(e) => {
                fmt.write_fmt(format_args!("CertificateError - {}", e))?;
            }
            SwishError::SwishHttpError(e) => {
                fmt.write_fmt(format_args!("SwishHttpError - {}", e))?;
            }
        }
        Ok(())
    }
}

pub struct SwishClient {
    maybe_sign_cert_pkcs12: Option<ParsedPkcs12>,
    client: reqwest::Client,
    base_url: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Version {
    V1,
    V2,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Method {
    Get,
    Put,
    Post,
    Patch,
}

impl SwishClient {
    async fn construct<'a, P1: AsRef<std::path::Path>, P2: AsRef<std::path::Path>>(
        auth_cert_path: P1,
        auth_cert_pass: &'a str,
        sign_cert_path: Option<P2>,
        sign_cert_pass: Option<&'a str>,
        base_url: &'static str,
    ) -> Result<Self> {
        // Read the auth cert file
        let mut auth_cert_buf = Vec::new();
        let mut f = match tokio::fs::File::open(auth_cert_path).await {
            Ok(f) => f,
            Err(e) => {
                return Err(SwishError::FileSystemError(format!(
                    "Could not open certificate file due to {:?}",
                    e
                )));
            }
        };
        match f.read_to_end(&mut auth_cert_buf).await {
            Ok(_) => (),
            Err(e) => {
                return Err(SwishError::FileSystemError(format!(
                    "Could not read certificate file due to {:?}",
                    e
                )));
            }
        };

        let auth_cert = match reqwest::Identity::from_pkcs12_der(&auth_cert_buf, auth_cert_pass) {
            Ok(r) => r,
            Err(e) => {
                return Err(SwishError::FileSystemError(format!(
                    "Could not create identity due to {:?}",
                    e
                )))
            }
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let client = match reqwest::ClientBuilder::new()
            .default_headers(headers)
            .https_only(true)
            .identity(auth_cert)
            .use_native_tls()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                return Err(SwishError::Unspecified(format!(
                    "Could not create reqwest client due to: {:?}",
                    e
                )))
            }
        };

        let maybe_sign_cert_pkcs12 = if let Some(sign_cert_path) = sign_cert_path {
            if let Some(sign_cert_pass) = sign_cert_pass {
                // Read the auth cert file
                let mut sign_cert_buf = Vec::new();
                let mut f = match tokio::fs::File::open(sign_cert_path).await {
                    Ok(f) => f,
                    Err(e) => {
                        return Err(SwishError::FileSystemError(format!(
                            "Could not open certificate file due to {:?}",
                            e
                        )));
                    }
                };
                match f.read_to_end(&mut sign_cert_buf).await {
                    Ok(_) => (),
                    Err(e) => {
                        return Err(SwishError::FileSystemError(format!(
                            "Could not read certificate file due to {:?}",
                            e
                        )));
                    }
                };

                let sign_cert_pkcs12 = Pkcs12::from_der(&sign_cert_buf).map_err(|_| {
                    SwishError::CertificateError(format!(
                        "Could not authentication Pkcs#12 from der-file"
                    ))
                })?;
                let sign_cert_pkcs12 = sign_cert_pkcs12.parse(sign_cert_pass).map_err(|_| {
                    SwishError::CertificateError(format!("Could not parse authentication Pkcs#12"))
                })?;
                Some(sign_cert_pkcs12)
            } else {
                return Err(SwishError::CertificateError(format!(
                    "A signature certificate path was provided without a password"
                )));
            }
        } else {
            None
        };

        let c = Self {
            client,
            base_url,
            maybe_sign_cert_pkcs12,
        };
        Ok(c)
    }

    pub async fn test_client<'a, P: AsRef<std::path::Path>>(
        auth_cert_path: P,
        auth_cert_pass: &'a str,
        sign_cert_path: Option<P>,
        sign_cert_pass: Option<&'a str>,
    ) -> Result<Self> {
        let base_url = "https://mss.cpc.getswish.net/swish-cpcapi/api";
        Self::construct(
            auth_cert_path,
            auth_cert_pass,
            sign_cert_path,
            sign_cert_pass,
            base_url,
        )
        .await
    }

    /// Create a new client for interacting with swish. Requires a certificate path and password.
    pub async fn new<'a, P: AsRef<std::path::Path>>(
        auth_cert_path: P,
        auth_cert_pass: &'a str,
        sign_cert_path: Option<P>,
        sign_cert_pass: Option<&'a str>,
    ) -> Result<Self> {
        let base_url = "https://cpc.getswish.net/swish-cpcapi/api";
        Self::construct(
            auth_cert_path,
            auth_cert_pass,
            sign_cert_path,
            sign_cert_pass,
            base_url,
        )
        .await
    }

    async fn send_basic(
        &self,
        url: &str,
        body: impl Serialize,
        method: Method,
    ) -> Result<reqwest::Response> {
        let client = match method {
            Method::Get => self.client.get(url),
            Method::Put => self.client.put(url),
            Method::Post => self.client.post(url),
            Method::Patch => self.client.patch(url),
        };
        let resp = match client.json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                return Err(SwishError::NetworkError(format!(
                    "Could not send message due to {:?}",
                    e
                )))
            }
        };
        if resp.status() == 422 {
            let status = resp.status();
            let text = match resp.text().await {
                Ok(r) => r,
                Err(e) => {
                    return Err(SwishError::Unspecified(format!(
                        "Could not get text from http error: {}",
                        e
                    )))
                }
            };
            let api_error_objects: Vec<ApiErrorObject> =
                serde_json::from_str(&text).map_err(|e| {
                    SwishError::SerializationError(format!(
                        "Could not deserialize error message: {}",
                        e
                    ))
                })?;
            return Err(SwishError::ApiError(ApiError {
                code: status.as_u16(),
                errors: api_error_objects,
            }));
        } else if resp.status() != 200 && resp.status() != 201 {
            return Err(SwishError::SwishHttpError(format!(
                "Code - {}, body - {}",
                resp.status(),
                match resp.text().await {
                    Ok(t) => t,
                    Err(_) => format!("Error body missing"),
                }
            )));
        }
        Ok(resp)
    }

    async fn send<'a, T: DeserializeOwned>(
        &self,
        url: &str,
        body: impl Serialize,
        method: Method,
    ) -> Result<T> {
        let resp = self.send_basic(url, body, method).await?;
        let body: T = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                return Err(SwishError::SerializationError(format!(
                    "Could not deserialize response due to {:?}",
                    e
                )))
            }
        };
        Ok(body)
    }

    async fn send_get_header(
        &self,
        url: &str,
        body: impl Serialize,
        method: Method,
    ) -> Result<reqwest::header::HeaderMap> {
        let resp = self.send_basic(url, body, method).await?;
        Ok(resp.headers().to_owned())
    }

    /// There are two versions of the API for creating the payment request. The first one serves the merchants
    /// that wish the payment request identifier to be generated by swish system. The second one serves the
    /// merchants that wish to provide this identifier when sending the request.
    pub async fn payment_request(
        &self,
        payment_request: PaymentRequest<'_>,
    ) -> Result<PaymentResponse> {
        let resp = match payment_request {
            PaymentRequest::V1(payment_request) => {
                let url = format!("{}/v1/paymentrequests", self.base_url);
                self.send_get_header(&url, payment_request, Method::Post)
                    .await?
            }
            PaymentRequest::V2(payment_request) => {
                let url = format!(
                    "{}/v2/paymentrequests/{}",
                    self.base_url, payment_request.id
                );
                self.send_get_header(&url, payment_request, Method::Put)
                    .await?
            }
        };
        let location = resp
            .get("location")
            .ok_or_else(|| SwishError::SerializationError(format!("'location' not found")))?
            .to_str()
            .map_err(|e| {
                SwishError::SerializationError(format!("'location' is not a string: {}", e))
            })?
            .to_owned();
        let payment_request_token = match resp.get("paymentrequesttoken") {
            Some(r) => Some(
                r.to_str()
                    .map_err(|e| {
                        SwishError::SerializationError(format!(
                            "'paymentrequesttoken' is not a string: {}",
                            e
                        ))
                    })?
                    .to_owned(),
            ),
            None => None,
        };
        let resp = PaymentResponse {
            location,
            payment_request_token,
        };
        return Ok(resp);
    }

    /// Get the current payment object for provided url, the url can be found in
    /// [PaymentResponse](PaymentResponse)
    pub async fn payment_retrieve(&self, location: String) -> Result<PaymentObject> {
        self.send(&location, "", Method::Get).await
    }

    /// Get the current payment object for provided id and version. The id can be extracted from
    /// the location in [PaymentResponse](PaymentResponse)
    pub async fn payment_retrieve_from_id(&self, id: &str) -> Result<PaymentObject> {
        let payment_retrieve_url = format!("{}/v1/paymentrequests/{}", self.base_url, id);
        self.send(&payment_retrieve_url, "", Method::Get).await
    }

    /// Until a payment request is accepted by the payer it can be retracted by the merchant with the cancel
    /// operation. The request is cancelable while its status is “CREATED”. In any other status, “ERROR”, “PAID”,
    /// “CANCELLED” etc. the cancel operation will return error code “RP07”.
    pub async fn cancel_payment_request(
        &self,
        payment_request_id: String,
        version: Version,
    ) -> Result<PaymentObject> {
        let version = match version {
            Version::V1 => "v1",
            Version::V2 => "v2",
        };
        let url = format!(
            "{}/{}/paymentrequests/{}",
            self.base_url, version, payment_request_id
        );
        self.send(&url, "", Method::Patch).await
    }

    pub async fn refund_request(
        &self,
        mut refund_request: RefundRequest<'_>,
        version: Version,
    ) -> Result<RefundResponse> {
        let (url, method) = match version {
            Version::V1 => {
                match refund_request.instruction_uuid.take() {
                    Some(_) => {
                        return Err(SwishError::VersionError(format!(
                            "Version 1 of this API does not allow an ID"
                        )))
                    }
                    None => (),
                };
                let url = format!("{}/v1/refunds/", self.base_url);
                (url, Method::Post)
            }
            Version::V2 => {
                let id = refund_request.instruction_uuid.take();
                let url = match id {
                    Some(id) => format!("{}/v2/refunds/{}", self.base_url, id),
                    None => {
                        return Err(SwishError::VersionError(format!(
                            "Version 2 of this API requires you provide an ID"
                        )))
                    }
                };
                (url, Method::Put)
            }
        };
        let headers = self.send_get_header(&url, refund_request, method).await?;
        Ok(RefundResponse {
            location: headers
                .get("Location")
                .ok_or_else(|| {
                    SwishError::SerializationError(format!("Could not get location from response"))
                })?
                .to_str()
                .map_err(|_| {
                    SwishError::SerializationError(format!("Could not get location from response"))
                })?
                .to_owned(),
        })
    }

    pub async fn refund_retrieve(&self, location: String) -> Result<RefundObject> {
        self.send(&location, "", Method::Get).await
    }

    /// Get the current refund object for provided id and version. The id can be extracted from
    /// the location in [RefundResponse](RefundResponse)
    pub async fn refund_retrieve_from_id(&self, id: &str) -> Result<RefundObject> {
        let refund_retrieve_url = format!("{}/v1/refunds/{}", self.base_url, id);
        self.send(&refund_retrieve_url, "", Method::Get).await
    }

    pub async fn payout_request(
        &self,
        payout_request: PayoutRequest<'_>,
    ) -> Result<PayoutResponse> {
        let url = format!("{}/v1/payouts/", self.base_url);
        let headers = self
            .send_get_header(&url, payout_request, Method::Post)
            .await?;
        match headers.get("location") {
            Some(location) => Ok(PayoutResponse {
                location: location
                    .to_str()
                    .map_err(|_| {
                        SwishError::SerializationError(format!(
                            "Could not convert response header to string"
                        ))
                    })?
                    .to_owned(),
            }),
            None => Err(SwishError::SerializationError(format!(
                "Could not get location from payout request"
            ))),
        }
    }

    pub async fn payout_request_string(
        &self,
        payout_request: StringPayoutRequest<'_>,
    ) -> Result<PayoutResponse> {
        let url = format!("{}/v1/payouts/", self.base_url);
        self.send(&url, payout_request, Method::Post).await
    }

    /// Constructs a payout request from the certificate provided to the client.
    /// This is a helper method for signing the payout request correctly.
    pub fn construct_payout_request<'a>(
        &self,
        payout_instruction_uuid: &'a str,
        payer_payment_reference: &'a str,
        payer_alias: &'a str,
        payee_alias: &'a str,
        payee_ssn: &'a str,
        amount: Decimal,
        currency: Currency,
        payout_type: PayoutType,
        message: String,
        callback_url: Option<&'a str>,
    ) -> Result<PayoutRequest<'a>> {
        let pkcs12 = self.maybe_sign_cert_pkcs12.as_ref().ok_or_else(|| {
            SwishError::CertificateError(format!("No signing certificate provided"))
        })?;
        if message.len() >= 50 {
            return Err(SwishError::Unspecified(format!(
                "Message has to be shorter than 50 characters long. The provided message is {}: {}",
                message.len(),
                message
            )));
        }
        let instruction_date = format!("{}", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S"));
        let signing_certificate_serial_number = pkcs12
            .cert
            .serial_number()
            .to_bn()
            .map_err(|_| {
                SwishError::CertificateError(format!(
                    "Could not convert certificate serial number to BN"
                ))
            })?
            .to_hex_str()
            .map_err(|_| {
                SwishError::CertificateError(format!(
                    "Could not convert serial number to hex string"
                ))
            })?
            .to_string();
        let payload = Payload {
            payout_instruction_uuid,
            payer_payment_reference,
            payer_alias,
            payee_alias,
            payee_ssn,
            amount,
            currency,
            payout_type,
            instruction_date,
            signing_certificate_serial_number,
            message,
        };
        let string_payload = serde_json::to_string(&payload).map_err(|_| {
            SwishError::SerializationError(format!("Could not serialize the payload object"))
        })?;
        let hash = sha512(string_payload.as_bytes());
        let mut signer = Signer::new(MessageDigest::sha512(), &pkcs12.pkey)
            .map_err(|_| SwishError::CertificateError(format!("Could not generate signer")))?;
        signer
            .update(&hash)
            .map_err(|_| SwishError::CertificateError(format!("Could not update signer")))?;
        let signature = signer
            .sign_to_vec()
            .map_err(|_| SwishError::CertificateError(format!("Could not sign")))?;
        let signature = openssl::base64::encode_block(&signature);
        let payout_req = PayoutRequest {
            payload,
            callback_url,
            signature,
        };
        Ok(payout_req)
    }

    pub async fn payout_retrieve(&self, location: String) -> Result<PayoutObject> {
        self.send(&location, "", Method::Get).await
    }

    /// Get the current payout object for provided id. The id can be extracted from
    /// the location in [PayoutResponse](PayoutResponse)
    pub async fn payout_retrieve_from_id(&self, id: &str) -> Result<PayoutObject> {
        let payout_retrieve_url = format!("{}/v1/payouts/{}", self.base_url, id);
        self.send(&payout_retrieve_url, "", Method::Get).await
    }
}

/// The currency to use. The only currently supported value is
/// SEK.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Currency {
    SEK,
}

/// The status of the transaction. Possible values: CREATED,
/// PAID, DECLINED, ERROR.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PaymentStatus {
    CREATED,
    PAID,
    DECLINED,
    ERROR,
}

// Payment
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentObject {
    /// Payment request ID.
    pub id: String,
    /// Payment reference of the payee, which is the merchant that
    /// receives the payment. This reference could be order id or
    /// similar. Allowed characters are a z A Z 0 9 _.+*/ and length
    /// must be between 1 and 36 characters.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payee_payment_reference: Option<String>,
    /// Payment reference, from the bank, of the payment that occurred
    /// based on the Payment request. Only available if status is PAID.
    pub payment_reference: String,
    /// URL that Swish will use to notify caller about the outcome of
    /// the Payment request. The URL has to use HTTPS.
    pub callback_url: String,
    /// The registered cellphone number of the person that makes the
    /// payment. It can only contain numbers and has to be at least 8
    /// and at most 15 numbers. It also needs to match the following
    /// format in order to be found in Swish: country code + cellphone
    /// number (without leading zero). E.g.: 46712345678
    pub payer_alias: String,
    /// The social security number of the individual making the
    /// payment, should match the registered value for payerAlias or
    /// the payment will not be accepted.
    /// The value should be a proper Swedish social security number
    /// (personnummer or sammordningsnummer).
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payer_ssn: Option<String>,
    /// Minimum age (in years) that the individual connected to the
    /// payerAlias has to be in order for the payment to be accepted.
    /// Value has to be in the range of 1 to 99.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub age_limit: Option<String>,
    /// The Swish number of the payee.
    pub payee_alias: String,
    /// The amount of money to pay. The amount cannot be less than
    /// 0.01 SEK and not more than 999999999999.99 SEK. Valid
    /// value has to be all numbers or with 2 digit decimal separated by
    /// a period.
    pub amount: Decimal,
    /// The currency to use. The only currently supported value is
    /// SEK.
    pub currency: Currency,
    /// Merchant supplied message about the payment/order. Max 50
    /// chars. Allowed characters are the letters a ö, A Ö, the numbers
    /// 9 and the special characters :;.,?!()”.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub message: Option<String>,
    /// The status of the transaction. Possible values: CREATED,
    /// PAID, DECLINED, ERROR.
    pub status: PaymentStatus,
    /// The time and date that the payment request was created.
    pub date_created: DateTime<Utc>,
    /// The time and date that the payment request was paid. Only
    /// applicable if status was PAID.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub date_paid: Option<DateTime<Utc>>,
    /// A code indicating what type of error occurred. Only applicable
    /// if status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub error_code: Option<String>,
    /// A descriptive error message (in English) indicating what type
    /// of error occurred. Only applicable if status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub error_message: Option<String>,
    /// Additional information about the error. Only applicable if status
    /// is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub additional_information: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentRequestV1<'a> {
    /// The registered cellphone number of the person that makes the
    /// payment. It can only contain numbers and has to be at least 8
    /// and at most 15 numbers. It also needs to match the following
    /// format in order to be found in Swish: country code + cellphone
    /// number (without leading zero). E.g.: 46712345678
    pub callback_url: &'a str,
    /// The Swish number of the payee.
    pub payee_alias: &'a str,
    /// The amount of money to pay. The amount cannot be less than
    /// 0.01 SEK and not more than 999999999999.99 SEK. Valid
    /// value has to be all numbers or with 2 digit decimal separated by
    /// a period.
    pub amount: Decimal,
    /// The currency to use. The only currently supported value is
    /// SEK.
    pub currency: Currency,

    /// Payment reference of the payee, which is the merchant that
    /// receives the payment. This reference could be order id or
    /// similar. Allowed characters are a z A Z 0 9 _.+*/ and length
    /// must be between 1 and 36 characters.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payee_payment_reference: Option<&'a str>,
    /// The registered cellphone number of the person that makes the
    /// payment. It can only contain numbers and has to be at least 8
    /// and at most 15 numbers. It also needs to match the following
    /// format in order to be found in Swish: country code + cellphone
    /// number (without leading zero). E.g.: 46712345678
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payer_alias: Option<&'a str>,
    /// Merchant supplied message about the payment/order. Max 50
    /// chars. Allowed characters are the letters a ö, A Ö, the numbers
    /// 9 and the special characters :;.,?!()”.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub message: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentRequestV2<'a> {
    /// Payment request ID.
    #[serde(skip_serializing)]
    pub id: &'a str,
    /// Payment reference of the payee, which is the merchant that
    /// receives the payment. This reference could be order id or
    /// similar. Allowed characters are a z A Z 0 9 _.+*/ and length
    /// must be between 1 and 36 characters.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payee_payment_reference: Option<&'a str>,
    ///URL that Swish will use to notify caller about the outcome of
    ///the Payment request. The URL has to use HTTPS.
    pub callback_url: &'a str,
    /// The registered cellphone number of the person that makes the
    /// payment. It can only contain numbers and has to be at least 8
    /// and at most 15 numbers. It also needs to match the following
    /// format in order to be found in Swish: country code + cellphone
    /// number (without leading zero). E.g.: 46712345678
    ///
    /// If this is set then the request will be a m-commerce payment, else it will be an e-commerce
    /// payment. m-commerce payments will get a payment request token back with the response.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payer_alias: Option<&'a str>,
    /// The social security number of the individual making the
    /// payment, should match the registered value for payerAlias or
    /// the payment will not be accepted.
    /// The value should be a proper Swedish social security number
    /// (personnummer or sammordningsnummer).
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payer_ssn: Option<&'a str>,
    /// Minimum age (in years) that the individual connected to the
    /// payerAlias has to be in order for the payment to be accepted.
    /// Value has to be in the range of 1 to 99.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payer_age_limit: Option<usize>,
    /// The Swish number of the payee.
    pub payee_alias: &'a str,
    /// The amount of money to pay. The amount cannot be less than
    /// 0.01 SEK and not more than 999999999999.99 SEK. Valid
    /// value has to be all numbers or with 2 digit decimal separated by
    /// a period.
    pub amount: Decimal,
    /// The currency to use. The only currently supported value is
    /// SEK.
    pub currency: Currency,
    /// Merchant supplied message about the payment/order. Max 50
    /// chars. Allowed characters are the letters a ö, A Ö, the numbers
    /// 9 and the special characters :;.,?!()”.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub message: Option<String>,
}

/// Returned response when creating a payment request
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentResponse {
    /// A URL for retrieving the status of the payment request.
    pub location: String,
    /// Returned when creating an m-commerce payment request. The
    /// token to use when opening the Swish app.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payment_request_token: Option<String>,
}

/// Enum for either version of the payment request API
pub enum PaymentRequest<'a> {
    V1(PaymentRequestV1<'a>),
    V2(PaymentRequestV2<'a>),
}

// Refund
/// The status of the refund transaction. Possible values:
/// VALIDATED Refund ongoing DEBITED Money has been withdrawn from your account
/// PAID The payment was successful ERROR An error occurred.
/// See list of error codes for all potential error conditions.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RefundStatus {
    VALIDATED,
    DEBITED,
    PAID,
    ERROR,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundObject {
    /// Refund ID.
    pub id: String,
    /// Payment reference supplied by the merchant. This could
    /// be order id or similar.
    pub payer_payment_reference: Option<String>,
    /// Reference of the original payment that this refund is for.
    pub original_payment_reference: String,
    /// Reference of the refund payment that occurred based on
    /// the created refund. Only available if status is PAID.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payment_reference: Option<String>,
    /// URL that Swish will use to notify caller about the
    /// outcome of the refund. The URL has to use HTTPS.
    pub callback_url: String,
    /// The Swish number of the merchant that makes the refund payment.
    pub payer_alias: String,
    /// The cellphone number of the person that receives the refund payment.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payee_alias: Option<String>,
    /// The amount of money to refund. The amount cannot be
    /// less than 0.01 SEK and not more than 999999999999.99
    /// SEK. Moreover, the amount cannot exceed the remaining
    /// amount of the original payment that the refund is for.
    pub amount: Decimal,
    /// The currency to use. The only currently supported value is SEK.
    pub currency: Currency,
    /// Merchant supplied message about the refund. Max 50
    /// chars. Allowed characters are the letters a ö, A Ö, the
    /// numbers 0 9 and the special characters :;.,?!()”.
    pub message: String,
    /// The status of the refund transaction. Possible values:
    /// VALIDATED Refund ongoing DEBITED Money has been withdrawn from your account
    /// PAID The payment was successful ERROR An error occurred.
    /// See list of error codes for all potential error conditions.
    pub status: PaymentStatus,
    /// The time and date that the payment refund was created.
    pub date_created: DateTime<Utc>,
    /// The time and date that the payment refund was paid.
    pub date_paid: DateTime<Utc>,
    /// A code indicating what type of error occurred. Only
    /// applicable if status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub error_code: Option<String>,
    /// A descriptive error message (in English) indicating what
    /// type of error occurred. Only applicable if status is
    /// ERROR
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub error_message: Option<String>,
    /// Additional information about the error. Only applicable if
    /// status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub additional_information: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundRequest<'a> {
    /// Payment reference supplied by the merchant. This could
    /// be order id or similar.
    /// This is not used by Swish but is
    /// included in responses back to the client.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payer_payment_reference: Option<&'a str>,

    /// Reference of the original payment that this refund is for.
    pub original_payment_reference: &'a str,

    /// URL that Swish will use to notify caller about the
    /// outcome of the refund. The URL has to use HTTPS.
    pub callback_url: &'a str,

    /// The Swish number of the merchant that makes the refund
    /// payment.
    pub payer_alias: &'a str,

    /// The Cell phone number of the person that
    /// receives the refund payment.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payee_alias: Option<&'a str>,

    /// The amount of money to refund. The amount cannot be
    /// less than 0.01 SEK and not more than 999999999999.99
    /// SEK. Moreover, the amount cannot exceed the remaining
    /// amount of the original payment that the refund is for.
    pub amount: Decimal,

    /// The currency to use. The only currently supported value is
    /// SEK.
    pub currency: Currency,

    /// Merchant supplied message about the refund. Max 50
    /// chars. Allowed characters are the letters a ö, A Ö, the
    /// numbers 0 9 and the special characters :;.,?!()”.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub message: Option<String>,

    /// This is only necessary when using version 2 of the Swish API. It should be `None` in
    /// version 1. This is not sent via the payload but rather is supplied in the version 2 URL.
    ///
    /// The instructionUUID of the URL should be a unique identifier (UUID) created/generated by the
    /// caller and confirm to format ^[0-9A-F]{32}$, that is a 32 character hexadecimal number (upper case)
    /// represented as a string. The same instructionUUID will be used in the response message
    /// Location header property.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub instruction_uuid: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RefundResponse {
    /// URL that Swish will use to notify caller about the
    /// outcome of the refund. The URL has to use HTTPS.
    pub location: String,
}

// Payout
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PayoutType {
    Payout,
}

/// CREATED, INITIATED, BIR_PAYMENT_INITIATED, DEBITED, PAID, ERROR. The status of the payout request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PayoutStatus {
    CREATED,
    INITIATED,
    BIRPAYMENTINITIATED,
    DEBIT,
    PAID,
    ERROR,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PayoutObject {
    /// 100.00 Amount to be paid. Only period/dot (”.”) are accepted as decimal character with maximum 2 digits after.
    /// Digits after separator are optional.
    pub amount: Decimal,
    /// SEK The currency to use. The only currently supported value is SEK.
    pub currency: Currency,
    /// YYYY MM DDThh:mm:ssTZD The time and date that the payout request was created.
    pub instruction_date: String,
    /// Alphanumeric, 0 50 chars. Custom message.
    pub message: String,
    /// Numeric, 8 15 digits The Swish number of the payee.
    /// No preceding “+” or zeros should be added. It should always be started with country code.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub payee_alias: Option<String>,
    /// Payment reference, from the bank, of the payment that occurred
    /// based on the Payment request. Only available if status is PAID.
    pub payment_reference: String,
    /// UUID 32 hexadecimal (16 based) digits. An identifier created by the merchant to
    /// uniquely identify a payout instruction sent to the Swish system.
    /// Swish uses this identifier to guarantee the uniqueness of a payout
    /// instruction and prevent occurrence of unintended double payments.
    #[serde(rename = "payoutInstructionUUID")]
    pub payout_instruction_uuid: String,
    /// 35 characters. Valid characters are: a zA Z0 _.+*/ Merchant specific reference.
    /// This reference could be order id or similar.
    pub payer_payment_reference: String,
    /// URL that Swish will use to notify caller about the outcome of the refund. The URL has to use HTTPS.
    pub callback_url: String,
    /// Numeric, 10 digits The merchant Swish number that makes the payment.
    pub payer_alias: String,
    /// YYYYMMDDnnnn The social security number of the individual receiving the payout,
    /// should match the registered value for payeeAlias or the payout will not be accepted.
    /// The value should be a proper Swedish social security number (personnummer or sammordningsnummer).
    /// 12 digit SSN of the payee.
    #[serde(rename = "payeeSSN")]
    pub payee_ssn: String,
    /// PAYOUT Currently only “PAYOUT” is allowed – meaning immediate payout.
    pub payout_type: PayoutType,
    /// CREATED, INITIATED, BIR_PAYMENT_INITIATED, DEBITED, PAID, ERROR. The status of the payout request.
    pub status: PayoutStatus,
    /// Serial number of the certificate in hexadecimal format (without the leading ‘0x’).
    /// Max length 64 digits. The public key of the certificate will be used to verify the signature
    pub signing_certificate_serial_number: String,
    /// YYYY MM DDThh:mm:ssTZD The time and date that the payout request was created.
    pub date_created: DateTime<Utc>,
    /// The time and date that the payout request was paid. Only applicable if status was PAID.
    pub date_paid: DateTime<Utc>,
    /// A code indicating what type of error occurred. Only applicable if status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub error_code: Option<String>,
    /// A descriptive error message (in English) indicating what type of
    /// error occurred. Only applicable if status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub error_message: Option<String>,
    /// Additional information about the error. Only applicable if status is ERROR.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub additional_information: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Payload<'a> {
    /// Example: 100.05
    /// The amount to be paid.
    /// Note that only period/dot (”.”) are accepted
    /// as decimal character with maximal 2 digits
    /// after. Digits after separator are optional.
    pub amount: Decimal,

    /// The only supported value is: SEK
    /// The currency to use.
    pub currency: Currency,

    /// YYYY-MM-DDTHH:MM:SS
    /// Date and time for when the payout
    /// instruction was supplied.
    /// Example: 2019-12-03T11:07:16
    pub instruction_date: String,

    /// 0-50 Alphanumeric characters.
    /// Custom message.
    /// Note: For MSS, an error simulation code
    /// can be set in this property in order to
    /// simulate an error situation. Refer to section
    /// 9.5 for available codes.
    pub message: String,

    /// The mobile phone number of the person
    /// that receives the payment.
    pub payee_alias: &'a str,

    /// YYYYMMDDXXXX
    /// The Social Security Number of the person
    /// that receives the payment.
    #[serde(rename = "payeeSSN")]
    pub payee_ssn: &'a str,

    /// Numeric, 10-11 digits
    /// The Swish number of the merchant that
    /// makes the payout payment.
    pub payer_alias: &'a str,

    /// 1-35 Alphanumeric characters.
    /// A Merchant specific reference. This
    /// reference could for example be order id or
    /// similar.
    /// The property is not used by Swish but is
    /// included in responses back to the client.
    pub payer_payment_reference: &'a str,

    /// A UUID of length 32. All upper case
    /// hexadecimal characters.
    /// A unique identifier created by the merchant
    /// to uniquely identify a payout instruction sent
    /// to the Swish system. Swish uses this
    /// identifier to guarantee the uniqueness of the
    /// payout instruction and prevent occurrences
    /// of unintended double payments.
    #[serde(rename = "payoutInstructionUUID")]
    pub payout_instruction_uuid: &'a str,

    /// Only supported value is: PAYOUT
    /// Immediate payout.
    pub payout_type: PayoutType,

    /// Serial number of the signing certificate in
    /// hexadecimal format (without any leading ‘0x’
    /// characters).
    /// The public key of the certificate with this
    /// serial number will be used to verify the
    /// signature.
    pub signing_certificate_serial_number: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
/// A payout request object which has the payload already in a json string format in order to
/// guarantee that no modification happens after the signature has been created
pub struct StringPayoutRequest<'a> {
    /// Payload contains most of the fields for a payout request
    /// It has already been turned into a JSON string in order to guarantee that no modification is
    /// made after being signed
    pub payload: String,

    /// https://<host[:port]>/...
    /// URL that Swish system will use to notify
    /// caller about the result of the payment
    /// request. The URL must use HTTPS.
    /// If not set (or not provided in the payload) it
    /// is the responsibility of the caller to check the
    /// status of the request using GET operation
    /// as described in chapter 10.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub callback_url: Option<&'a str>,

    /// Base64 encoded.
    /// Signature of the hashed payload.
    pub signature: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PayoutRequest<'a> {
    /// Payload contains most of the fields for a payout request
    pub payload: Payload<'a>,

    /// https://<host[:port]>/...
    /// URL that Swish system will use to notify
    /// caller about the result of the payment
    /// request. The URL must use HTTPS.
    /// If not set (or not provided in the payload) it
    /// is the responsibility of the caller to check the
    /// status of the request using GET operation
    /// as described in chapter 10.
    #[serde(skip_serializing_if = "is_none")]
    #[serde(default)]
    pub callback_url: Option<&'a str>,

    /// Base64 encoded.
    /// Signature of the hashed payload.
    pub signature: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PayoutResponse {
    pub location: String,
}

fn is_none<T>(o: &Option<T>) -> bool {
    o.is_none()
}
