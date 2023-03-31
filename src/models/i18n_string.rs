use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct I18nString {
    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub l: Option<String>, // http://en.wikipedia.org/wiki/IETF_language_tag

    pub s: String,
}
