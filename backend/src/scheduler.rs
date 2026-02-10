use chrono::{Timelike, Utc};
use reqwest::Client;
use sqlx::PgPool;
use std::time::Duration;

use crate::db;
use crate::email::{self, EmailConfig};
use crate::models::NewsletterConfig;
use crate::openclaw_client::{self, OpenClawConfig};

pub fn run_scheduler(
    pool: PgPool,
    openclaw: OpenClawConfig,
    email_config: EmailConfig,
) {
    tokio::spawn(async move {
        let client = Client::new();
        let check_interval = Duration::from_secs(60 * 5);
        loop {
            tokio::time::sleep(check_interval).await;
            if let Err(e) = run_tick(&pool, &client, &openclaw, &email_config).await {
                eprintln!("scheduler tick error: {}", e);
            }
        }
    });
}

async fn run_tick(
    pool: &PgPool,
    client: &Client,
    openclaw: &OpenClawConfig,
    email_config: &EmailConfig,
) -> Result<(), String> {
    let configs = db::list_active_newsletter_configs(pool)
        .await
        .map_err(|e| e.to_string())?;
    for config in configs {
        if !is_due(pool, &config).await.map_err(|e| e.to_string())? {
            continue;
        }
        run_one(pool, client, openclaw, email_config, &config).await?;
    }
    Ok(())
}

async fn is_due(pool: &PgPool, config: &NewsletterConfig) -> Result<bool, sqlx::Error> {
    let last = db::get_last_run_at(pool, config.id).await?;
    let now = Utc::now();
    let today = now.date_naive();
    if let Some(last_run) = last {
        if last_run.date_naive() >= today {
            return Ok(false);
        }
    }
    let send_time = config.send_time_utc;
    let send_mins = send_time.hour() * 60 + send_time.minute();
    let now_mins = now.hour() * 60 + now.minute();
    Ok(now_mins >= send_mins && now_mins < send_mins + 15)
}

async fn run_one(
    pool: &PgPool,
    client: &Client,
    openclaw: &OpenClawConfig,
    email_config: &EmailConfig,
    config: &NewsletterConfig,
) -> Result<(), String> {
    let body = match openclaw_client::generate_newsletter(client, openclaw, config).await {
        Ok(b) => b,
        Err(e) => {
            let _ = db::insert_run_log(
                pool,
                config.id,
                "failure",
                Some(&e),
                None,
            )
            .await;
            return Err(e);
        }
    };
    let subject = format!("{} – {}", config.title, Utc::now().format("%Y-%m-%d"));
    if let Err(e) = email::send_newsletter(
        email_config,
        &config.delivery_email,
        &subject,
        &body,
    )
    .await
    {
        let _ = db::insert_run_log(
            pool,
            config.id,
            "failure",
            Some(&e),
            None,
        )
        .await;
        return Err(e);
    }
    db::insert_run_log(pool, config.id, "success", None, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
