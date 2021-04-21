use chrono::{DateTime, Utc};
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
    pub account_id: i64,
    pub name: String,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub fields: Option<serde_json::Map<String, serde_json::Value>>,
    pub expires: Option<DateTime<Utc>>,
}
