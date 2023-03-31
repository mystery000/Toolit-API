// NOTE: We set an unusually high recursion limit in order to allow warp to have a lot of endpoints
#![recursion_limit = "256"]
#![type_length_limit = "2000000"]
use appinsights::{InMemoryChannel, TelemetryClient, TelemetryConfig};
use lazy_static::lazy_static;
use rust_decimal::Decimal;
use std::time::Duration;
use warp::{http::Method, Filter};
mod api;
mod models;
use models::*;
mod fault;
mod filters;
mod push;
mod test_utils;
mod util;
#[macro_use]
extern crate bitflags;

#[cfg(debug_assertions)]
lazy_static! {
    static ref PRODUCTION_ENVIRONMENT: bool = false;
    static ref ACCESS_TOKEN_SECRET: String = String::from("access-token-secret");
    static ref REFRESH_TOKEN_SECRET: String = String::from("refresh-token-secret");
    static ref COSMOS_MASTER_KEY: String = String::from(
        "jBqbt1R780nirFckloRlOXa0qMj3qSVPg1sdZlC9Zak0qutQqVEXNdn7Sk9CNalilU1U8ZmEiz92doHbaa8rsw=="
    );
    static ref COSMOS_ACCOUNT: String = String::from("toolit-play");
    static ref STORAGE_ACCOUNT: String = String::from("toolitlive");
    static ref STORAGE_MASTER_KEY: String = String::from(
        "QYUhuHbiX2FHlrLkhIWgXqx2nDQIZ0yIJr+KQL5a1rgEYfcK8AQWHYS3tCykUHe9ehg2YwjekhKkRfYDa34Ltg=="
    );
    static ref SENDGRID_API_KEY: String = std::env::var("SENDGRID_API_KEY").unwrap();
    static ref NOTIFICATION_HUB_ACCOUNT: push::Account = push::Account {
        key_name: "notification-hub-name".to_string(),
        key: "notification-hub-key".to_string(),
    };
    static ref CERTIFICATE_STORAGE_CONTAINER: String = std::env::var("CERTIFICATE_STORAGE_CONTAINER").unwrap();

    // TODO(Jonathan): Fill in certificate information
    static ref BANKID_CERT_PATH: String = String::from(
        "trust_server_certificate.txt"
        );
    static ref BANKID_IDENT_PATH: String = String::from(
        "./Keystore_Toolit_20210420.p12"
        );
    static ref BANKID_IDENT_PASS: String = std::env::var("BANKID_IDENT_PASS").unwrap();
    // NOTE: Needs to be exactly 32 bytes long
    static ref BANKID_NID_SECRET: String = String::from("fdw&/rewHLasWlqtp/f7qwNU23fdfBsQ");
    // TODO(Jonathan): Fill in certificate information
    static ref SWISH_CERT_PATH: String = std::env::var("SWISH_CERT_PATH").unwrap();
    static ref SWISH_CERT_PASS: String = std::env::var("SWISH_CERT_PASS").unwrap();
    static ref SWISH_INTERMEDIATE_ACCOUNT_NUMBER: String = String::from("1234914271");
    static ref BASE_CALLBACK_URL: String = String::from("https://toolit-api-play.azurewebsites.net");

    static ref APPLICATION_INSIGHTS_INSTRUMENTATION_KEY: String =
        String::from("117686a5-04ca-4767-a2ea-26d3083de43e");
    static ref APPLICATION_INSIGHTS_INGESTION_ENDPOINT: String =
        String::from("https://norwayeast-0.in.applicationinsights.azure.com/v2/track");
    static ref APPLICATION_INSIGHTS_TELEMETRY_CLIENT: TelemetryClient<InMemoryChannel> = {
        let config = TelemetryConfig::builder()
            .i_key(APPLICATION_INSIGHTS_INSTRUMENTATION_KEY.to_string())
            .interval(Duration::from_secs(2))
            .endpoint(APPLICATION_INSIGHTS_INGESTION_ENDPOINT.to_string())
            .build();
        TelemetryClient::<InMemoryChannel>::from_config(config)
    };

    static ref BROKERAGE_PERCENTAGE: Decimal = Decimal::new(4, 2);
}

#[cfg(not(debug_assertions))]
lazy_static! {
    static ref PRODUCTION_ENVIRONMENT: bool =
        std::env::var("PRODUCTION_ENVIRONMENT").unwrap() == "true";
    static ref ACCESS_TOKEN_SECRET: String = std::env::var("ACCESS_TOKEN_SECRET").unwrap();
    static ref REFRESH_TOKEN_SECRET: String = std::env::var("REFRESH_TOKEN_SECRET").unwrap();
    static ref COSMOS_MASTER_KEY: String = std::env::var("COSMOS_MASTER_KEY").unwrap();
    static ref COSMOS_ACCOUNT: String = std::env::var("COSMOS_ACCOUNT").unwrap();
    static ref STORAGE_ACCOUNT: String = std::env::var("STORAGE_ACCOUNT").unwrap();
    static ref SENDGRID_API_KEY: String = std::env::var("SENDGRID_API_KEY").unwrap();
    static ref STORAGE_MASTER_KEY: String = std::env::var("STORAGE_MASTER_KEY").unwrap();
    static ref NOTIFICATION_HUB_ACCOUNT: push::Account = push::Account {
        key_name: "DefaultFullSharedAccessSignature".to_string(),
        key: std::env::var("PUSH_NOTIFICATION_HUB_KEY").unwrap(),
    };
    static ref CERTIFICATE_STORAGE_CONTAINER: String = std::env::var("CERTIFICATE_STORAGE_CONTAINER").unwrap();

    // TODO(Jonathan): Fill in certificate information
    static ref BANKID_CERT_PATH: String = std::env::var("BANKID_CERT_PATH").unwrap();
    static ref BANKID_IDENT_PATH: String = std::env::var("BANKID_IDENT_PATH").unwrap();
    static ref BANKID_IDENT_PASS: String = std::env::var("BANKID_IDENT_PASS").unwrap();
    // NOTE: Needs to be exactly 32 bytes long
    static ref BANKID_NID_SECRET: String = std::env::var("BANKID_NID_SECRET").unwrap();
    // TODO(Jonathan): Fill in certificate information
    static ref SWISH_CERT_PATH: String = std::env::var("SWISH_CERT_PATH").unwrap();
    static ref SWISH_CERT_PASS: String = std::env::var("SWISH_CERT_PASS").unwrap();
    static ref SWISH_INTERMEDIATE_ACCOUNT_NUMBER: String = std::env::var("SWISH_INTERMEDIATE_ACCOUNT_NUMBER").unwrap();
    static ref BASE_CALLBACK_URL: String = std::env::var("BASE_CALLBACK_URL").unwrap();

    static ref APPLICATION_INSIGHTS_INSTRUMENTATION_KEY: String =
        std::env::var("APPLICATION_INSIGHTS_INSTRUMENTATION_KEY").unwrap();
    static ref APPLICATION_INSIGHTS_INGESTION_ENDPOINT: String =
        std::env::var("APPLICATION_INSIGHTS_INGESTION_ENDPOINT").unwrap();
    static ref APPLICATION_INSIGHTS_TELEMETRY_CLIENT: TelemetryClient<InMemoryChannel> = {
        let config = TelemetryConfig::builder()
            .i_key(APPLICATION_INSIGHTS_INSTRUMENTATION_KEY.to_string())
            .interval(Duration::from_secs(2))
            .endpoint(APPLICATION_INSIGHTS_INGESTION_ENDPOINT.to_string())
            .build();
        TelemetryClient::<InMemoryChannel>::from_config(config)
    };

    static ref BROKERAGE_PERCENTAGE: Decimal = Decimal::new(4, 2);
}

// NOTE(Jonathan): This is so that we can box (higher runtime cost lower
// compiletime) when compiling and not box when compiling the release.
#[cfg(debug_assertions)]
macro_rules! maybe_box {
    ($expression:expr) => {
        $expression.boxed()
    };
}

#[cfg(not(debug_assertions))]
macro_rules! maybe_box {
    ($expression:expr) => {
        $expression.boxed()
    };
}

const USER_COLLECTION: &str = "users";
const OFFICE_COLLECTION: &str = "offices";
const CRAFTSMAN_COLLECTION: &str = "craftsmen";
const CRAFTSMAN_NOTE_COLLECTION: &str = "craftsman_notes";
const PAYMENT_COLLECTION: &str = "payments";
const CHAT_COLLECTION: &str = "chats";
const MESSAGE_COLLECTION: &str = "messages";
const TASK_COLLECTION: &str = "tasks";
const BID_COLLECTION: &str = "bids";
const AUTH_EMAIL_COLLECTION: &str = "auth_emails";
const AUTH_NID_COLLECTION: &str = "auth_nids";
const AD_COLLECTION: &str = "ads";

fn routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let chats = warp::path("chats");
    let messages = warp::path("messages");
    let users = warp::path("users");
    let offices = warp::path("offices");
    let craftsmen = warp::path("craftsmen");
    let notes = warp::path("notes");
    let crafts = warp::path("crafts");
    let ratings = warp::path("ratings");
    let payments = warp::path("payments");
    let tasks = warp::path("tasks");
    let bids = warp::path("bids");
    let password = warp::path("password");
    let ads = warp::path("ads");

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[
            Method::OPTIONS,
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_headers(vec![
            "User-Agent",
            "Authorization",
            "Referer",
            "Origin",
            "Access-Control-Request-Method",
            "Access-Control-Request-Headers",
            "Accept",
            "Range",
            "If-Range",
            "Content-Type",
            "Content-Length",
        ])
        .max_age(600);
    let user_get = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::user_get));
    let user_delete = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::user_delete));
    let user_put = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::user_put));
    let user_image_put = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path("image"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 16)) // 16 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 16)) // 16 mb.
        .and_then(api::user_image_put));
    let signup = maybe_box!(users
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_version())
        .and_then(api::signup));
    let bankid = maybe_box!(users
        .and(warp::path("bankid"))
        .and(warp::path("se"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::filters::addr::remote())
        .and(filters::with_version())
        .and_then(api::bankid));
    let signin = maybe_box!(users
        .and(warp::path("signin"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_version())
        .and_then(api::signin));
    let refresh_token = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path("token"))
        .and(warp::path("refresh"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_version())
        .and_then(api::refresh_token));
    let user_poll = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path("poll"))
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(filters::with_range())
        .and(filters::with_since())
        .and_then(api::user_poll));
    let office_poll = maybe_box!(offices
        .and(warp::path::param())
        .and(warp::path("poll"))
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(filters::with_range())
        .and(filters::with_since())
        .and_then(api::office_poll));
    let change_password = maybe_box!(users
        .and(warp::path::param())
        .and(password)
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_optional_token())
        .and(filters::with_version())
        .and_then(api::change_password));
    let forgot_password = maybe_box!(users
        .and(password)
        .and(warp::path("forgot"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_version())
        .and_then(api::forgot_password));
    let user_roles_put = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path("roles"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::user_roles_put));
    let user_device_post = maybe_box!(users
        .and(warp::path::param())
        .and(warp::path("devices"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::user_device_post));
    let office_post = maybe_box!(offices
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::office_post));
    let office_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::office_delete));
    let offices_get_all = maybe_box!(offices
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::offices_get_all));
    let office_get = maybe_box!(offices
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::office_get));
    let office_find = maybe_box!(offices
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::office_find));
    let craftsman_note_post = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(notes)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_note_post));
    let craftsman_note_put = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(notes)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_note_put));
    let craftsman_note_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(notes)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_note_delete));
    let craftsman_post = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_post));
    let craft_apply = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(crafts)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craft_apply));
    let craft_approve = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(crafts)
        .and(warp::path::param())
        .and(warp::path("approve"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craft_approve));
    let craft_reject = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(crafts)
        .and(warp::path::param())
        .and(warp::path("reject"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craft_reject));
    let craftsman_freeze = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(warp::path("freeze"))
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_freeze));
    let craftsman_put = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_put));
    let craft_certificate_put = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(crafts)
        .and(warp::path::param())
        .and(warp::path("certificate"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 16)) // 16 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 16)) // 16 mb.
        .and_then(api::craft_certificate_put));
    let craftsman_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_delete));
    let craftsman_get = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_get));
    let craftsman_task_finish = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path("craftsman_finish"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::craftsman_task_finish));
    let rating_post = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(ratings)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::rating_post));
    let rating_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(craftsmen)
        .and(warp::path::param())
        .and(ratings)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::rating_delete));
    //let rating_get = maybe_box!(offices
    //    .and(warp::path::param())
    //    .and(craftsmen)
    //    .and(warp::path::param())
    //    .and(ratings)
    //    .and(warp::path::param())
    //    .and(warp::path::end())
    //    .and(warp::get())
    //    .and(filters::with_token())
    //    .and(filters::with_version())
    //    .and_then(api::rating_get));
    //let payment_init = maybe_box!(offices
    //    .and(warp::path::param())
    //    .and(tasks)
    //    .and(warp::path::param())
    //    .and(bids)
    //    .and(warp::path::param())
    //    .and(payments)
    //    .and(warp::path::end())
    //    .and(warp::post())
    //    .and(warp::body::json())
    //    .and(filters::with_token())
    //    .and(filters::with_version())
    //    .and_then(api::payment_init));
    // NOTE: This can be called by anyone but we callback the swish server to verify the payment
    let payment_escrow = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(payments)
        .and(warp::path::param())
        .and(warp::path("escrow"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and_then(api::payment_escrow));
    // NOTE: This is a post since that's what Swish needs, this endpoint should only be called by
    // swish
    // FIXME(Jonathan): This is only for payout
    //let payment_finalize = maybe_box!(offices
    //    .and(warp::path::param())
    //    .and(tasks)
    //    .and(warp::path::param())
    //    .and(bids)
    //    .and(warp::path::param())
    //    .and(payments)
    //    .and(warp::path::param())
    //    .and(payments)
    //    .and(warp::path("finalize"))
    //    .and(warp::path::end())
    //    .and(warp::post())
    //    .and(warp::body::json())
    //    .and_then(api::payment_finalize));
    let payment_mark_paid = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(payments)
        .and(warp::path::param())
        .and(warp::path("mark_paid"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::payment_mark_paid));
    let payment_refund_init = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(payments)
        .and(warp::path::param())
        .and(warp::path("refund"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::payment_refund_init));
    let payment_refund_finish = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(payments)
        .and(warp::path::param())
        .and(warp::path("refund"))
        .and(warp::path::param())
        .and(warp::path("finish"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and_then(api::payment_refund_finish));
    let payment_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(payments)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::payment_delete));
    let payment_get = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(payments)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::payment_get));
    let task_post = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::task_post));
    let task_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::task_put));
    let task_image_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path("image"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 16)) // 16 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 16)) // 16 mb.
        .and_then(api::task_image_put));
    let task_video_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path("video"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 300)) // 300 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 300)) // 300 mb.
        .and_then(api::task_video_put));
    let task_status_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path("status"))
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::task_status_put));
    let task_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::task_delete));
    let task_get = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::task_get));
    let task_finish = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(warp::path("finish"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::task_finish));
    let bid_post = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::bid_post));
    let bid_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::bid_put));
    let bid_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::bid_delete));
    let bid_get = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::bid_get));
    let bid_accept = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(warp::path("accept"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::bid_accept));
    let bid_cancel = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(warp::path("cancel"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::bid_cancel));
    let message_post = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::message_post));
    let message_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::message_put));
    let message_status_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::param())
        .and(warp::path("status"))
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::message_status_put));
    let message_image_put = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::param())
        .and(warp::path("image"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 16)) // 16 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 16)) // 16 mb.
        .and_then(api::message_image_put));
    let message_put_read = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::param())
        .and(warp::path("read"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::message_put_read));
    let chat_put_read = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(warp::path("read"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::chat_put_read));
    let message_delete = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::message_delete));
    let message_get = maybe_box!(offices
        .and(warp::path::param())
        .and(tasks)
        .and(warp::path::param())
        .and(bids)
        .and(warp::path::param())
        .and(chats)
        .and(warp::path::param())
        .and(messages)
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::message_get));
    let ad_post = maybe_box!(ads
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::ad_post));
    let ad_video_put = maybe_box!(ads
        .and(warp::path::param())
        .and(warp::path("video"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 300)) // 300 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 300)) // 300 mb.
        .and_then(api::ad_video_put));
    let ad_image_put = maybe_box!(ads
        .and(warp::path::param())
        .and(warp::path("image"))
        .and(warp::path::end())
        .and(warp::put())
        .and(filters::with_token())
        .and(filters::with_version())
        .and(warp::body::content_length_limit(1024 * 1000 * 16)) // 16 mb.
        .and(warp::filters::multipart::form().max_length(1024 * 1000 * 16)) // 16 mb.
        .and_then(api::ad_image_put));
    let ad_put = maybe_box!(ads
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::put())
        .and(warp::body::json())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::ad_put));
    let ad_delete = maybe_box!(ads
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::delete())
        .and(filters::with_token())
        .and(filters::with_version())
        .and_then(api::ad_delete));

    // Required by Azure health checks.
    let main = warp::path::end().map(|| warp::reply());

    // Used by CORS preflight requests.
    let options = warp::any().and(warp::options()).map(|| warp::reply());

    let routes = maybe_box!(main
        .or(user_get)
        .or(user_delete)
        .or(user_put)
        .or(user_image_put)
        .or(user_roles_put)
        .or(user_device_post)
        .or(signin)
        .or(signup)
        .or(bankid)
        .or(refresh_token)
        .or(user_poll)
        .or(office_poll)
        .or(forgot_password)
        .or(change_password)
        .or(office_post)
        .or(office_delete)
        .or(office_get)
        .or(offices_get_all)
        .or(office_find)
        .or(craftsman_note_post)
        .or(craftsman_note_put)
        .or(craftsman_note_delete)
        .or(craftsman_post)
        .or(craft_apply)
        .or(craft_approve)
        .or(craft_reject)
        .or(craftsman_put)
        .or(craftsman_freeze)
        .or(craft_certificate_put)
        .or(craftsman_delete)
        .or(craftsman_get)
        .or(craftsman_task_finish)
        .or(rating_post)
        .or(rating_delete)
        //.or(rating_get)
        .or(payment_escrow)
        //FIXME(Jonathan): This is only for payout
        //.or(payment_finalize)
        .or(payment_refund_init)
        .or(payment_refund_finish)
        .or(payment_mark_paid)
        .or(payment_delete)
        .or(payment_get)
        .or(task_post)
        .or(task_put)
        .or(task_delete)
        .or(task_get)
        .or(task_image_put)
        .or(task_video_put)
        .or(task_status_put)
        .or(task_finish)
        .or(bid_post)
        .or(bid_put)
        .or(bid_delete)
        .or(bid_get)
        .or(bid_accept)
        .or(bid_cancel)
        .or(message_post)
        .or(message_put)
        .or(message_image_put)
        .or(message_put_read)
        .or(chat_put_read)
        .or(message_status_put)
        .or(message_delete)
        .or(message_get)
        .or(ad_post)
        .or(ad_video_put)
        .or(ad_image_put)
        .or(ad_put)
        .or(ad_delete)
        .or(options)
        .recover(filters::handle_rejection)
        .with(&cors));

    routes
}

#[tokio::main]
async fn main() {
    let routes = routes();

    if cfg!(debug_assertions) {
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await
    } else {
        warp::serve(routes).run(([0, 0, 0, 0], 3030)).await
    }
}

// #[cfg(test)]
// mod api_test {
//     use super::test_utils::*;
//     #[ignore]
//     #[tokio::test]
//     async fn signup_test() {
//         test_env();
//         user_signup("moot@meme.org", "tester").await;
//     }

//     #[tokio::test]
//     async fn user_poll_test() {
//         test_env();
//         let (access, _refresh, id) = signin_access_refresh_id("moot@meme.org", "tester").await;
//         let _up = user_poll(&access, &id).await;
//     }

//     #[tokio::test]
//     async fn filled_user_poll_test() {
//         test_env();
//         let (access, _refresh, id) = signin_access_refresh_id("moot@meme.org", "tester").await;
//         let _up = user_poll(&access, &id).await;
//     }
// }
