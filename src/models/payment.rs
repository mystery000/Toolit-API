use crate::models::PaymentMethod;
use crate::util;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    SEK,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PaymentState {
    // Payments that have been initialized but not recieved money
    Initialized,
    // Payments that have been refunded
    Refunded,
    // Refund failed
    RefundFailed,
    // Payments that have failed
    Failed,
    // Payments that have been confirmed paid to escrow
    PaidToEscrow,
    // Payments that have been finalized
    Finalized,
    // Payments that have been paid to craftsmen
    PaidToCraftsman,
    // State the payment should only be in due to some error in logic. Should not be reachable by
    // well functioning code.
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    pub office_id: String,

    pub task_id: String,

    pub bid_id: String,

    pub craftsman_id: String,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub swish_payment_id: Option<String>,

    pub payment_state: PaymentState,

    pub payment_method: PaymentMethod,

    // When the swish payment was provided to the escrow
    pub payment_date: Option<DateTime<Utc>>,

    pub amount: Decimal,

    pub currency: Currency,

    pub modified: DateTime<Utc>,
}
