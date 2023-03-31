use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthEmail {
    pub id: String,

    pub passhash: Option<String>,

    pub user_id: String,
}
