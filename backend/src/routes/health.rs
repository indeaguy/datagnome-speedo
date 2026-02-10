use rocket::get;
use rocket::serde::json::Json;
use rocket::State;
use std::collections::HashMap;

use crate::supabase::SupabaseClient;

#[get("/health")]
pub async fn health(supabase: &State<SupabaseClient>) -> Json<HashMap<&'static str, &'static str>> {
    let mut m = HashMap::new();
    m.insert("status", "ok");
    if !supabase.health_check().await {
        m.insert("status", "db_error");
    }
    Json(m)
}
