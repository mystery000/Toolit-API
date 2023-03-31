use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rating {
    #[serde(default)]
    pub id: String,

    pub office_id: String,

    pub craftsman_id: String,

    pub user_id: String,

    pub task_id: String,

    pub amount: i32,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub text: Option<String>,

    pub header: String,

    pub created: DateTime<Utc>,

    pub modified: DateTime<Utc>,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,
}
