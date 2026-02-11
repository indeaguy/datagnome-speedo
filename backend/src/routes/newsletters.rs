use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

use crate::auth::{ApprovedUser, User};
use crate::email::{self, EmailConfig};
use crate::models::{CreateNewsletterConfig, UpdateNewsletterConfig};
use crate::openclaw_client::{self, OpenClawConfig};
use crate::supabase::SupabaseClient;

#[rocket::get("/me/approval-status")]
pub async fn approval_status(
    user: User,
    supabase: &State<SupabaseClient>,
) -> Result<Json<serde_json::Value>, Status> {
    let approved = supabase
        .is_user_approved(user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(serde_json::json!({ "approved": approved })))
}

#[rocket::get("/me/newsletters")]
pub async fn list(user: ApprovedUser, supabase: &State<SupabaseClient>) -> Result<Json<Vec<serde_json::Value>>, Status> {
    let configs = supabase
        .list_newsletters_by_user(user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let out: Vec<serde_json::Value> = configs.into_iter().map(|c| c.into_api_response()).collect();
    Ok(Json(out))
}

#[rocket::post("/me/newsletters", data = "<body>")]
pub async fn create(
    user: ApprovedUser,
    supabase: &State<SupabaseClient>,
    body: Json<CreateNewsletterConfig>,
) -> Result<Json<serde_json::Value>, Status> {
    let email = body
        .delivery_email
        .as_deref()
        .or(user.0.email.as_deref())
        .ok_or(Status::BadRequest)?;
    let config = supabase
        .create_newsletter(user.0.user_id, email, &body)
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(config.into_api_response()))
}

#[rocket::get("/me/newsletters/<id>")]
pub async fn get(user: ApprovedUser, supabase: &State<SupabaseClient>, id: &str) -> Result<Json<serde_json::Value>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let config = supabase
        .get_newsletter_by_id(id, user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    Ok(Json(config.into_api_response()))
}

#[rocket::put("/me/newsletters/<id>", data = "<body>")]
pub async fn update(
    user: ApprovedUser,
    supabase: &State<SupabaseClient>,
    id: &str,
    body: Json<UpdateNewsletterConfig>,
) -> Result<Json<serde_json::Value>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let config = supabase
        .update_newsletter(id, user.0.user_id, &body)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    Ok(Json(config.into_api_response()))
}

#[rocket::delete("/me/newsletters/<id>")]
pub async fn delete(user: ApprovedUser, supabase: &State<SupabaseClient>, id: &str) -> Result<Status, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let ok = supabase
        .delete_newsletter(id, user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if ok {
        Ok(Status::NoContent)
    } else {
        Err(Status::NotFound)
    }
}

#[rocket::options("/me/newsletters/<_id>/send-sample")]
pub fn send_sample_options(_id: &str) -> Status {
    Status::NoContent
}

#[rocket::post("/me/newsletters/<id>/send-sample", data = "<overlay>")]
pub async fn send_sample(
    user: ApprovedUser,
    supabase: &State<SupabaseClient>,
    openclaw: &State<OpenClawConfig>,
    email_config: &State<EmailConfig>,
    client: &State<reqwest::Client>,
    id: &str,
    overlay: Option<Json<UpdateNewsletterConfig>>,
) -> Result<Json<serde_json::Value>, (Status, String)> {
    eprintln!("[send-sample] POST id={}", id);
    let id = Uuid::parse_str(id).map_err(|_| (Status::BadRequest, "Invalid newsletter id".into()))?;
    let mut config = supabase
        .get_newsletter_by_id(id, user.0.user_id)
        .await
        .map_err(|e| {
            eprintln!("[send-sample] get_newsletter_by_id failed: {}", e);
            (Status::InternalServerError, e)
        })?
        .ok_or((Status::NotFound, "Newsletter not found".into()))?;

    if let Some(ref body) = overlay {
        if let Some(t) = body.title.as_ref() {
            config.title = t.clone();
        }
        if let Some(t) = body.topics.as_ref() {
            config.topics = t.clone();
        }
        if let Some(t) = body.tone.as_ref() {
            config.tone = t.clone();
        }
        if let Some(l) = body.length.as_ref() {
            config.length = l.clone();
        }
        if let Some(e) = body.delivery_email.as_ref() {
            config.delivery_email = e.clone();
        }
        if let Some(f) = body.features.as_ref() {
            config.features = f.clone();
        }
    }

    let body = openclaw_client::generate_newsletter(client.inner(), openclaw.inner(), &config)
        .await
        .map_err(|e| {
            eprintln!("[send-sample] generate_newsletter failed: {}", e);
            (Status::UnprocessableEntity, e)
        })?;

    let body = body.trim();
    if body.is_empty() {
        eprintln!("[send-sample] OpenClaw returned empty content");
        return Err((
            Status::UnprocessableEntity,
            "OpenClaw did not return any content. Check the agent and gateway.".into(),
        ));
    }

    let subject = format!("{} – Sample – {}", config.title, Utc::now().format("%Y-%m-%d %H:%M"));
    email::send_newsletter(
        email_config.inner(),
        &config.delivery_email,
        &subject,
        body,
    )
    .await
    .map_err(|e| {
        eprintln!("[send-sample] send_newsletter failed: {}", e);
        (Status::InternalServerError, e)
    })?;

    eprintln!("[send-sample] sent to {}", config.delivery_email);
    Ok(Json(serde_json::json!({ "sent": true })))
}
