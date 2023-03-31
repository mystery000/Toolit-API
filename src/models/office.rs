use crate::models::I18nString;
use crate::util;
use chrono::{DateTime, Utc};
use geojson::GeoJson;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Office {
    #[serde(default)]
    pub id: String,

    #[serde(skip_serializing_if = "util::is_false")]
    #[serde(default)]
    pub deleted: bool,

    pub name: Vec<I18nString>,

    pub brokerage_percentage: Decimal,

    pub area: GeoJson,

    pub modified: DateTime<Utc>,
}
