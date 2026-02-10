use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NewsletterConfig {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub topics: Vec<String>,
    pub tone: String,
    pub length: String,
    pub send_time_utc: NaiveTime,
    pub timezone: String,
    pub delivery_email: String,
    pub is_active: bool,
    pub features: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNewsletterConfig {
    pub title: Option<String>,
    pub topics: Option<Vec<String>>,
    pub tone: Option<String>,
    pub length: Option<String>,
    pub send_time_utc: Option<String>,
    pub timezone: Option<String>,
    pub delivery_email: Option<String>,
    pub is_active: Option<bool>,
    pub features: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNewsletterConfig {
    pub title: Option<String>,
    pub topics: Option<Vec<String>>,
    pub tone: Option<String>,
    pub length: Option<String>,
    pub send_time_utc: Option<String>,
    pub timezone: Option<String>,
    pub delivery_email: Option<String>,
    pub is_active: Option<bool>,
    pub features: Option<serde_json::Value>,
}

impl NewsletterConfig {
    pub fn into_api_response(self) -> serde_json::Value {
        let send_time_utc = self.send_time_utc.format("%H:%M").to_string();
        serde_json::json!({
            "id": self.id,
            "user_id": self.user_id,
            "title": self.title,
            "topics": self.topics,
            "tone": self.tone,
            "length": self.length,
            "send_time_utc": send_time_utc,
            "timezone": self.timezone,
            "delivery_email": self.delivery_email,
            "is_active": self.is_active,
            "features": self.features,
            "created_at": self.created_at.to_rfc3339(),
            "updated_at": self.updated_at.to_rfc3339(),
        })
    }
}
