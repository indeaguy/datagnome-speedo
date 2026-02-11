//! Supabase REST API client for newsletter_config and newsletter_run_log.
//! Uses HTTPS only (no direct Postgres), so works on VPS with no DB port/DNS.

use chrono::{DateTime, NaiveTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::{CreateNewsletterConfig, NewsletterConfig, UpdateNewsletterConfig};

#[derive(Clone)]
pub struct SupabaseClient {
    base_url: String,
    key: String,
    client: Client,
}

/// Row shape from PostgREST (send_time_utc is "HH:MM:SS" string).
#[derive(Debug, Deserialize)]
struct NewsletterConfigRow {
    id: Uuid,
    user_id: Uuid,
    title: String,
    topics: Vec<String>,
    tone: String,
    length: String,
    #[serde(deserialize_with = "deser_time")]
    send_time_utc: NaiveTime,
    timezone: String,
    delivery_email: String,
    is_active: bool,
    features: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn deser_time<'de, D>(d: D) -> Result<NaiveTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    let parts: Vec<&str> = s.trim_end_matches('Z').split(':').collect();
    let h: u32 = parts.get(0).and_then(|p| p.parse().ok()).unwrap_or(0);
    let m: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let sec: u32 = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
    NaiveTime::from_hms_opt(h, m, sec).ok_or_else(|| serde::de::Error::custom("invalid time"))
}

impl NewsletterConfigRow {
    fn into_config(self) -> NewsletterConfig {
        NewsletterConfig {
            id: self.id,
            user_id: self.user_id,
            title: self.title,
            topics: self.topics,
            tone: self.tone,
            length: self.length,
            send_time_utc: self.send_time_utc,
            timezone: self.timezone,
            delivery_email: self.delivery_email,
            is_active: self.is_active,
            features: self.features,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl SupabaseClient {
    pub fn new(base_url: String, service_role_key: String) -> Self {
        let client = Client::new();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            key: service_role_key,
            client,
        }
    }

    fn rest_url(&self, path: &str) -> String {
        format!("{}/rest/v1/{}", self.base_url, path.trim_start_matches('/'))
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            reqwest::header::ACCEPT,
            "application/json".parse().unwrap(),
        );
        h.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        h.insert(
            "apikey",
            self.key.parse().unwrap(),
        );
        h.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.key).parse().unwrap(),
        );
        h
    }

    pub async fn list_newsletters_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<NewsletterConfig>, String> {
        let url = format!(
            "{}?user_id=eq.{}&order=created_at.desc&select=*",
            self.rest_url("newsletter_config"),
            user_id
        );
        let res = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Supabase list: {}", res.status()));
        }
        let rows: Vec<NewsletterConfigRow> = res.json().await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.into_config()).collect())
    }

    pub async fn get_newsletter_by_id(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<NewsletterConfig>, String> {
        let url = format!(
            "{}?id=eq.{}&user_id=eq.{}&select=*",
            self.rest_url("newsletter_config"),
            id,
            user_id
        );
        let res = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Supabase get: {}", res.status()));
        }
        let rows: Vec<NewsletterConfigRow> = res.json().await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().next().map(|r| r.into_config()))
    }

    pub async fn create_newsletter(
        &self,
        user_id: Uuid,
        delivery_email: &str,
        body: &CreateNewsletterConfig,
    ) -> Result<NewsletterConfig, String> {
        let title = body.title.as_deref().unwrap_or("");
        let topics = body.topics.clone().unwrap_or_default();
        let tone = body.tone.as_deref().unwrap_or("neutral").to_string();
        let length = body.length.as_deref().unwrap_or("medium").to_string();
        let send_time_utc = parse_time(body.send_time_utc.as_deref()).unwrap_or_else(|| {
            NaiveTime::from_hms_opt(9, 0, 0).unwrap()
        });
        let timezone = body.timezone.as_deref().unwrap_or("UTC").to_string();
        let is_active = body.is_active.unwrap_or(true);
        let features = body.features.clone().unwrap_or(serde_json::json!({}));

        let payload = serde_json::json!({
            "user_id": user_id,
            "title": title,
            "topics": topics,
            "tone": tone,
            "length": length,
            "send_time_utc": send_time_utc.format("%H:%M:%S").to_string(),
            "timezone": timezone,
            "delivery_email": delivery_email,
            "is_active": is_active,
            "features": features,
        });

        let url = format!("{}?select=*", self.rest_url("newsletter_config"));
        let res = self
            .client
            .post(&url)
            .headers(self.headers())
            .header("Prefer", "return=representation")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("Supabase create: {} {}", status, body));
        }
        let rows: Vec<NewsletterConfigRow> = res.json().await.map_err(|e| e.to_string())?;
        rows.into_iter()
            .next()
            .map(|r| r.into_config())
            .ok_or_else(|| "Supabase create: no row returned".into())
    }

    pub async fn update_newsletter(
        &self,
        id: Uuid,
        user_id: Uuid,
        body: &UpdateNewsletterConfig,
    ) -> Result<Option<NewsletterConfig>, String> {
        let existing = self.get_newsletter_by_id(id, user_id).await?;
        let Some(mut row) = existing else {
            return Ok(None);
        };
        if let Some(t) = body.title.as_ref() {
            row.title = t.clone();
        }
        if let Some(t) = body.topics.as_ref() {
            row.topics = t.clone();
        }
        if let Some(t) = body.tone.as_ref() {
            row.tone = t.clone();
        }
        if let Some(l) = body.length.as_ref() {
            row.length = l.clone();
        }
        if let Some(s) = body.send_time_utc.as_deref() {
            if let Some(t) = parse_time(Some(s)) {
                row.send_time_utc = t;
            }
        }
        if let Some(z) = body.timezone.as_ref() {
            row.timezone = z.clone();
        }
        if let Some(e) = body.delivery_email.as_ref() {
            row.delivery_email = e.clone();
        }
        if let Some(a) = body.is_active {
            row.is_active = a;
        }
        if let Some(f) = body.features.as_ref() {
            row.features = f.clone();
        }

        let payload = serde_json::json!({
            "title": row.title,
            "topics": row.topics,
            "tone": row.tone,
            "length": row.length,
            "send_time_utc": row.send_time_utc.format("%H:%M:%S").to_string(),
            "timezone": row.timezone,
            "delivery_email": row.delivery_email,
            "is_active": row.is_active,
            "features": row.features,
            "updated_at": Utc::now().to_rfc3339(),
        });

        let url = format!(
            "{}?id=eq.{}&user_id=eq.{}&select=*",
            self.rest_url("newsletter_config"),
            id,
            user_id
        );
        let res = self
            .client
            .patch(&url)
            .headers(self.headers())
            .header("Prefer", "return=representation")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Supabase update: {}", res.status()));
        }
        let rows: Vec<NewsletterConfigRow> = res.json().await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().next().map(|r| r.into_config()))
    }

    pub async fn delete_newsletter(&self, id: Uuid, user_id: Uuid) -> Result<bool, String> {
        let url = format!(
            "{}?id=eq.{}&user_id=eq.{}",
            self.rest_url("newsletter_config"),
            id,
            user_id
        );
        let res = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(res.status() == reqwest::StatusCode::NO_CONTENT || res.status().is_success())
    }

    pub async fn list_active_newsletter_configs(&self) -> Result<Vec<NewsletterConfig>, String> {
        let url = format!(
            "{}?is_active=eq.true&select=*",
            self.rest_url("newsletter_config")
        );
        let res = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Supabase list_active: {}", res.status()));
        }
        let rows: Vec<NewsletterConfigRow> = res.json().await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|r| r.into_config()).collect())
    }

    pub async fn get_last_run_at(
        &self,
        newsletter_config_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, String> {
        let url = format!(
            "{}?newsletter_config_id=eq.{}&select=run_at&order=run_at.desc&limit=1",
            self.rest_url("newsletter_run_log"),
            newsletter_config_id
        );
        let res = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Supabase get_last_run_at: {}", res.status()));
        }
        #[derive(Deserialize)]
        struct RunAtRow {
            run_at: DateTime<Utc>,
        }
        let rows: Vec<RunAtRow> = res.json().await.map_err(|e| e.to_string())?;
        Ok(rows.into_iter().next().map(|r| r.run_at))
    }

    pub async fn insert_run_log(
        &self,
        newsletter_config_id: Uuid,
        status: &str,
        error_message: Option<&str>,
        openclaw_response_id: Option<&str>,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "newsletter_config_id": newsletter_config_id,
            "status": status,
            "error_message": error_message,
            "openclaw_response_id": openclaw_response_id,
        });
        let url = self.rest_url("newsletter_run_log");
        let res = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("Supabase insert_run_log: {}", res.status()));
        }
        Ok(())
    }

    /// Minimal request to check Supabase REST is reachable.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}?select=id&limit=1", self.rest_url("newsletter_config"));
        self.client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// True if user_id exists in approved_users (backend uses service role, so RLS is bypassed).
    pub async fn is_user_approved(&self, user_id: Uuid) -> Result<bool, String> {
        let url = format!(
            "{}?user_id=eq.{}&select=user_id",
            self.rest_url("approved_users"),
            user_id
        );
        let res = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("approved_users lookup: {}", res.status()));
        }
        #[derive(Deserialize)]
        struct Row {
            #[allow(dead_code)]
            user_id: Uuid,
        }
        let rows: Vec<Row> = res.json().await.map_err(|e| e.to_string())?;
        Ok(!rows.is_empty())
    }
}

fn parse_time(s: Option<&str>) -> Option<NaiveTime> {
    let s = s?;
    let parts: Vec<&str> = s.split(':').collect();
    let h: u32 = parts.get(0)?.parse().ok()?;
    let m: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    NaiveTime::from_hms_opt(h, m, 0)
}
