use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Ad {
    pub id: String,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub image: Option<String>,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub video: Option<String>,

    pub title: String,

    pub text: String,

    pub company: String,

    pub url: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    pub modified: DateTime<Utc>,
}
