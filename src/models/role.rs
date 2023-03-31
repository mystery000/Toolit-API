use crate::models::RoleFlags;
use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    #[serde(with = "integer_representation")]
    pub flg: RoleFlags,

    #[serde(skip_serializing_if = "util::is_none")]
    #[serde(default)]
    pub sub: Option<String>,
}

mod integer_representation {
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    use crate::models::RoleFlags;
    type IntRep = u32;
    type Flags = RoleFlags;

    pub fn serialize<S>(date: &Flags, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        date.bits().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Flags, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw: IntRep = IntRep::deserialize(deserializer)?;
        RoleFlags::from_bits(raw).ok_or(serde::de::Error::custom(format!(
            "Unexpected flags value {}",
            raw
        )))
    }
}
