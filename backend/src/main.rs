mod auth;
mod email;
mod models;
mod openclaw_client;
mod routes;
mod scheduler;
mod supabase;

use rocket_cors::{AllowedOrigins, CorsOptions};

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let supabase_url = std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set (e.g. https://PROJECT_REF.supabase.co)");
    let supabase_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
        .expect("SUPABASE_SERVICE_ROLE_KEY must be set for backend REST API");
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET").ok();
    let jwt_audience = std::env::var("SUPABASE_JWT_AUDIENCE").ok();
    let jwt_config = auth::JwtConfig::from_env(
        jwt_secret.as_deref(),
        jwt_audience,
        None,
        Some(&supabase_url),
    ).expect("set SUPABASE_JWT_SECRET (legacy) or SUPABASE_URL for JWT signing keys");

    let supabase = supabase::SupabaseClient::new(supabase_url.clone(), supabase_key);

    let openclaw_url = std::env::var("OPENCLAW_GATEWAY_URL").unwrap_or_else(|_| String::new());
    let openclaw_token = std::env::var("OPENCLAW_GATEWAY_TOKEN").unwrap_or_else(|_| String::new());
    let openclaw_agent = std::env::var("OPENCLAW_AGENT_ID").unwrap_or_else(|_| "main".into());
    let openclaw_config = openclaw_client::OpenClawConfig {
        gateway_url: openclaw_url,
        token: openclaw_token,
        agent_id: openclaw_agent,
    };

    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| String::new());
    let smtp_port: u16 = std::env::var("SMTP_PORT").unwrap_or_else(|_| "587".into()).parse().unwrap_or(587);
    let smtp_user = std::env::var("SMTP_USER").unwrap_or_else(|_| String::new());
    let smtp_pass = std::env::var("SMTP_PASS").unwrap_or_else(|_| String::new());
    let smtp_from = std::env::var("SMTP_FROM").unwrap_or_else(|_| String::new());
    let email_config = email::EmailConfig {
        smtp_host,
        smtp_port,
        smtp_user,
        smtp_pass,
        from_address: smtp_from,
    };

    scheduler::run_scheduler(supabase.clone(), openclaw_config.clone(), email_config.clone());

    let cors_origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "*".into());
    let origins: AllowedOrigins = if cors_origins == "*" {
        AllowedOrigins::all()
    } else {
        AllowedOrigins::some_exact(
            &cors_origins.split(',').map(|s| s.trim()).collect::<Vec<_>>(),
        )
    };
    let cors = CorsOptions {
        allowed_origins: origins,
        ..CorsOptions::default()
    }
    .to_cors()
    .map_err(|e| format!("CORS config: {}", e))?;

    let http_client = reqwest::Client::new();
    let _ = rocket::build()
        .attach(cors)
        .manage(supabase)
        .manage(jwt_config)
        .manage(openclaw_config)
        .manage(email_config)
        .manage(http_client)
        .mount(
            "/api",
            rocket::routes![
                routes::health::health,
                routes::newsletters::approval_status,
                routes::newsletters::list,
                routes::newsletters::create,
                routes::newsletters::get,
                routes::newsletters::update,
                routes::newsletters::delete,
                routes::newsletters::send_sample,
            ],
        )
        .launch()
        .await?;
    Ok(())
}
