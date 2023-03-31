use crate::models::Role;
use crate::util;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub sub: String,

    #[serde(with = "jwt_numeric_date")]
    pub exp: DateTime<Utc>,

    #[serde(skip_serializing_if = "util::is_empty")]
    #[serde(default)]
    pub rol: Vec<Role>,
}

impl Claims {
    /// If a token should always be equal to its representation after serializing and deserializing
    /// again, this function must be used for construction. `DateTime` contains a microsecond field
    /// but JWT timestamps are defined as UNIX timestamps (seconds). This function normalizes the
    /// timestamps.
    pub fn new(sub: &str, exp: DateTime<Utc>, rol: &Vec<Role>) -> Self {
        let sub = String::from(sub);

        // normalize the timestamps by stripping of microseconds
        let exp = exp
            .date()
            .and_hms_milli(exp.hour(), exp.minute(), exp.second(), 0);
        Self {
            sub,
            exp,
            rol: rol.to_vec(),
        }
    }
}

mod jwt_numeric_date {
    // Custom serialization of DateTime<Utc> to conform with the JWT spec (RFC 7519 section 2, "Numeric Date").
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    // Serializes a DateTime<Utc> to a Unix timestamp (milliseconds since 1970/1/1T00:00:00T).
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let timestamp = date.timestamp();
        serializer.serialize_i64(timestamp)
    }

    // Attempts to deserialize an i64 and use as a Unix timestamp.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Utc.timestamp_opt(i64::deserialize(deserializer)?, 0)
            .single() // If there are multiple or no valid DateTimes from timestamp, return None.
            .ok_or_else(|| serde::de::Error::custom("invalid Unix timestamp value"))
    }
}
