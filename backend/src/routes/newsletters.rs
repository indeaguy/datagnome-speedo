use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

use crate::auth::{ApprovedUser, User};
use crate::models::{CreateNewsletterConfig, UpdateNewsletterConfig};
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
