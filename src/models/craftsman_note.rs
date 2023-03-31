use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CraftsmanNote {
    #[serde(default)]
    pub id: String,

    pub office_id: String,

    pub craftsman_id: String,

    pub text: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    pub modified: DateTime<Utc>,
}
