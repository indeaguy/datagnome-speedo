use chrono::NaiveTime;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateNewsletterConfig, NewsletterConfig, UpdateNewsletterConfig};

/// Rewrite DATABASE_URL to use the host's IPv4 address. Avoids "Network is unreachable"
/// on hosts that have no IPv6 route (e.g. some VPS) when the resolver returns AAAA first.
async fn database_url_prefer_ipv4(url: &str) -> Option<String> {
    let after_proto = url.split("://").nth(1)?;
    let authority = after_proto.split('/').next()?.split('?').next()?;
    let host_port = authority.rsplit('@').next()?;
    let lookup_addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{}:5432", host_port)
    };
    let addrs: Vec<_> = tokio::net::lookup_host(&lookup_addr).await.ok()?.collect();
    let ipv4 = addrs.into_iter().find(|a| a.is_ipv4())?;
    let replacement = format!("{}:{}", ipv4.ip(), ipv4.port());
    let new_url = url.replace(host_port, &replacement);
    if new_url == url {
        None
    } else {
        Some(new_url)
    }
}

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let connect_url = database_url_prefer_ipv4(database_url)
        .await
        .unwrap_or_else(|| database_url.to_string());
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&connect_url)
        .await
}

fn row_to_config(row: &sqlx::postgres::PgRow) -> Result<NewsletterConfig, sqlx::Error> {
    let topics: Vec<String> = row.try_get::<Vec<String>, _>("topics")?;
    Ok(NewsletterConfig {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        title: row.try_get("title")?,
        topics,
        tone: row.try_get("tone")?,
        length: row.try_get("length")?,
        send_time_utc: row.try_get("send_time_utc")?,
        timezone: row.try_get("timezone")?,
        delivery_email: row.try_get("delivery_email")?,
        is_active: row.try_get("is_active")?,
        features: row.try_get("features")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn list_newsletters_by_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<NewsletterConfig>, sqlx::Error> {
    let rows = sqlx::query(
        "select id, user_id, title, topics, tone, length, send_time_utc, timezone, delivery_email, is_active, features, created_at, updated_at from newsletter_config where user_id = $1 order by created_at desc",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_config).collect()
}

pub async fn get_newsletter_by_id(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Option<NewsletterConfig>, sqlx::Error> {
    let row = sqlx::query(
        "select id, user_id, title, topics, tone, length, send_time_utc, timezone, delivery_email, is_active, features, created_at, updated_at from newsletter_config where id = $1 and user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_to_config).transpose()
}

pub async fn create_newsletter(
    pool: &PgPool,
    user_id: Uuid,
    delivery_email: &str,
    body: &CreateNewsletterConfig,
) -> Result<NewsletterConfig, sqlx::Error> {
    let title = body.title.as_deref().unwrap_or("");
    let topics: Vec<String> = body.topics.clone().unwrap_or_default();
    let tone = body.tone.as_deref().unwrap_or("neutral").to_string();
    let length = body.length.as_deref().unwrap_or("medium").to_string();
    let send_time_utc = parse_time(body.send_time_utc.as_deref()).unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    let timezone = body.timezone.as_deref().unwrap_or("UTC").to_string();
    let is_active = body.is_active.unwrap_or(true);
    let features = body.features.clone().unwrap_or(serde_json::json!({}));

    let row = sqlx::query(
        "insert into newsletter_config (user_id, title, topics, tone, length, send_time_utc, timezone, delivery_email, is_active, features) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) returning id, user_id, title, topics, tone, length, send_time_utc, timezone, delivery_email, is_active, features, created_at, updated_at",
    )
    .bind(user_id)
    .bind(title)
    .bind(&topics)
    .bind(&tone)
    .bind(&length)
    .bind(send_time_utc)
    .bind(&timezone)
    .bind(delivery_email)
    .bind(is_active)
    .bind(&features)
    .fetch_one(pool)
    .await?;
    row_to_config(&row)
}

pub async fn update_newsletter(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    body: &UpdateNewsletterConfig,
) -> Result<Option<NewsletterConfig>, sqlx::Error> {
    let existing = get_newsletter_by_id(pool, id, user_id).await?;
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

    let updated = sqlx::query(
        "update newsletter_config set title = $2, topics = $3, tone = $4, length = $5, send_time_utc = $6, timezone = $7, delivery_email = $8, is_active = $9, features = $10 where id = $1 and user_id = $11 returning id, user_id, title, topics, tone, length, send_time_utc, timezone, delivery_email, is_active, features, created_at, updated_at",
    )
    .bind(id)
    .bind(&row.title)
    .bind(&row.topics)
    .bind(&row.tone)
    .bind(&row.length)
    .bind(row.send_time_utc)
    .bind(&row.timezone)
    .bind(&row.delivery_email)
    .bind(row.is_active)
    .bind(&row.features)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    updated.as_ref().map(row_to_config).transpose()
}

pub async fn delete_newsletter(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("delete from newsletter_config where id = $1 and user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_active_newsletter_configs(pool: &PgPool) -> Result<Vec<NewsletterConfig>, sqlx::Error> {
    let rows = sqlx::query(
        "select id, user_id, title, topics, tone, length, send_time_utc, timezone, delivery_email, is_active, features, created_at, updated_at from newsletter_config where is_active = true",
    )
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_to_config).collect()
}

pub async fn get_last_run_at(pool: &PgPool, newsletter_config_id: Uuid) -> Result<Option<chrono::DateTime<chrono::Utc>>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "select max(run_at) from newsletter_run_log where newsletter_config_id = $1",
    )
    .bind(newsletter_config_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn insert_run_log(
    pool: &PgPool,
    newsletter_config_id: Uuid,
    status: &str,
    error_message: Option<&str>,
    openclaw_response_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into newsletter_run_log (newsletter_config_id, status, error_message, openclaw_response_id) values ($1, $2, $3, $4)",
    )
    .bind(newsletter_config_id)
    .bind(status)
    .bind(error_message)
    .bind(openclaw_response_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn parse_time(s: Option<&str>) -> Option<NaiveTime> {
    let s = s?;
    let parts: Vec<&str> = s.split(':').collect();
    let h: u32 = parts.get(0)?.parse().ok()?;
    let m: u32 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    NaiveTime::from_hms_opt(h, m, 0)
}
