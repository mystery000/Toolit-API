use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(default)]
    pub handle: String,

    pub app_id: String,

    pub build: u32,

    pub os: String,

    pub os_ver: String,
}
