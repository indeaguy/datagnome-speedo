use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::User;
use crate::db;
use crate::models::{CreateNewsletterConfig, UpdateNewsletterConfig};

#[rocket::get("/me/newsletters")]
pub async fn list(user: User, pool: &State<PgPool>) -> Result<Json<Vec<serde_json::Value>>, Status> {
    let configs = db::list_newsletters_by_user(pool.inner(), user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let out: Vec<serde_json::Value> = configs.into_iter().map(|c| c.into_api_response()).collect();
    Ok(Json(out))
}

#[rocket::post("/me/newsletters", data = "<body>")]
pub async fn create(
    user: User,
    pool: &State<PgPool>,
    body: Json<CreateNewsletterConfig>,
) -> Result<Json<serde_json::Value>, Status> {
    let email = body
        .delivery_email
        .as_deref()
        .or(user.0.email.as_deref())
        .ok_or(Status::BadRequest)?;
    let config = db::create_newsletter(pool.inner(), user.0.user_id, email, &body)
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(config.into_api_response()))
}

#[rocket::get("/me/newsletters/<id>")]
pub async fn get(user: User, pool: &State<PgPool>, id: &str) -> Result<Json<serde_json::Value>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let config = db::get_newsletter_by_id(pool.inner(), id, user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    Ok(Json(config.into_api_response()))
}

#[rocket::put("/me/newsletters/<id>", data = "<body>")]
pub async fn update(
    user: User,
    pool: &State<PgPool>,
    id: &str,
    body: Json<UpdateNewsletterConfig>,
) -> Result<Json<serde_json::Value>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let config = db::update_newsletter(pool.inner(), id, user.0.user_id, &body)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    Ok(Json(config.into_api_response()))
}

#[rocket::delete("/me/newsletters/<id>")]
pub async fn delete(user: User, pool: &State<PgPool>, id: &str) -> Result<Status, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::BadRequest)?;
    let ok = db::delete_newsletter(pool.inner(), id, user.0.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if ok {
        Ok(Status::NoContent)
    } else {
        Err(Status::NotFound)
    }
}
