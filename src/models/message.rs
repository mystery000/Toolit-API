use crate::models::PublishStatus;
use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    pub office_id: String,

    pub task_id: String,

    pub bid_id: String,

    pub chat_id: String,

    pub user_id: String,

    pub sent: DateTime<Utc>,

    pub text: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub is_read: bool,

    pub publish_status: PublishStatus,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub image: Option<String>,

    pub modified: DateTime<Utc>,
}
