use chrono::{DateTime, Utc};
use ergo_database::object_id::AccountId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountType {
    pub account_type_id: String,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub account_id: AccountId,
    pub name: String,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub fields: Option<serde_json::Map<String, serde_json::Value>>,
    pub expires: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountPublicInfo {
    pub account_id: AccountId,
    pub account_type_id: String,
    pub name: String,
}
