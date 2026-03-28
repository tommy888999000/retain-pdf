use axum::Json;
use serde::Serialize;

use crate::models::ApiResponse;

#[derive(Serialize)]
pub struct HealthData {
    pub status: &'static str,
}

pub async fn health() -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::ok(HealthData { status: "ok" }))
}
