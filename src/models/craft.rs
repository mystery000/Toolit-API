use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CraftStatus {
    Applied,
    Approved,
    Rejected,
}

impl Default for CraftStatus {
    fn default() -> Self {
        CraftStatus::Applied
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Craft {
    pub id: String,
    #[serde(default)]
    pub status: CraftStatus,
    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub certificate_id: Option<String>,
    pub craft_type: CraftType,
}

impl Craft {
    pub fn swedish_name(&self) -> String {
        self.craft_type.swedish_name()
    }
}

impl CraftType {
    pub fn swedish_name(&self) -> String {
        match self {
            CraftType::Plumber => String::from("Rörmokare"),
            CraftType::Carpenter => String::from("Snickare"),
            CraftType::Electrician => String::from("Elektriker"),
            CraftType::Painter => String::from("Målare"),
            CraftType::FloorLayer => String::from("Golvläggare"),
            CraftType::Tiler => String::from("Plattläggare"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum CraftType {
    Plumber,
    Carpenter,
    Electrician,
    Painter,
    FloorLayer,
    Tiler,
}
