use rocket::get;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use std::collections::HashMap;

#[get("/health")]
pub async fn health(pool: &State<PgPool>) -> Json<HashMap<&'static str, &'static str>> {
    let mut m = HashMap::new();
    m.insert("status", "ok");
    if sqlx::query("select 1").fetch_one(pool.inner()).await.is_err() {
        m.insert("status", "db_error");
    }
    Json(m)
}
