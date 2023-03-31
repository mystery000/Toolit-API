use super::Craft;
use crate::models::Rating;
use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Craftsman {
    #[serde(default)]
    pub id: String,

    pub office_id: String,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub crafts: Vec<Craft>,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub frozen: bool,

    pub about_text: String,

    pub about_header: String,

    pub company_name: String,

    pub org_number: String,

    pub company_address: String,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub company_postal: Option<String>,

    pub work_area: String,

    pub completed_jobs: usize,

    pub member_since: DateTime<Utc>,

    pub craftsman_name: String,

    pub user_id: String,

    pub account_number: String,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub ratings: Vec<Rating>,

    pub f_tax: bool,

    pub modified: DateTime<Utc>,
}
