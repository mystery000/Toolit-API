use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    #[serde(default)]
    pub id: String,

    pub office_id: String,

    pub task_id: String,

    pub bid_id: String,

    pub modified: DateTime<Utc>,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,
}
