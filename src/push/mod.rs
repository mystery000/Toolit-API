const DIRECT_PUSH_URL: &str =
    "https://toolit-live.servicebus.windows.net/toolit/messages/?direct&api-version=2015-04";
const DIRECT_PUSH_ENCODED_URL: &str = "https%3A%2F%2Ftoolit-live.servicebus.windows.net%2Ftoolit%2Fmessages%2F%3Fdirect%26api-version%3D2015-04";

mod error;
pub use error::Error;

mod account;
pub use account::Account;

mod pns;
pub use pns::Pns;

mod send;
pub use send::send;

mod get_signature;
pub use get_signature::get_signature;

mod silent_push;
pub use silent_push::silent_push;

mod register_device_success;
pub use register_device_success::register_device_success;

mod send_custom_pn;
pub use send_custom_pn::send_custom_pn;
