use crate::models::CraftType;
use crate::models::PublishStatus;
use crate::util;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: String,

    pub office_id: String,

    pub user_id: String,

    pub crafts: Vec<CraftType>,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub images: Vec<String>,

    pub publish_status: PublishStatus,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub videos: Vec<String>,

    pub price: Decimal,

    pub address: String,

    pub city: String,

    pub postcode: String,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub date_done: Option<DateTime<Utc>>,

    pub description: String,

    pub title: String,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub accepted_bid: Option<String>,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub finished: bool,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub craftsman_indicated_finished: bool,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub rated: bool,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub payment_id: Option<String>,

    pub use_rot_rut: bool,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub realestate_union: Option<String>,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub apartment_number: Option<String>,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub property_designation: Option<String>,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub show_publicly: bool,

    pub modified: DateTime<Utc>,
}
