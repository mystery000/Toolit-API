use crate::models::Device;
use crate::util;
use crate::Role;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub test: bool,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub roles: Vec<Role>,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub devices: Vec<Device>,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub ratings: Vec<i32>,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub images: Vec<String>,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub preferred_name: Option<String>,

    pub last_name: String,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub middle_names: Option<String>,

    pub first_name: String,

    pub started: DateTime<Utc>,

    pub phone: String,

    pub address: String,

    pub email: String,

    pub nid: String,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub office_ids: Vec<String>,

    pub modified: DateTime<Utc>,
}

impl User {
    pub fn name<'a>(&'a self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}
