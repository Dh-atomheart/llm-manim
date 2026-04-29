use serde::Serialize;

use crate::types::response::AppResponse;

#[derive(Debug, Serialize)]
pub struct PingData {
    message: String,
}

#[tauri::command]
pub fn ping_backend() -> AppResponse<PingData> {
    AppResponse::ok(PingData {
        message: "pong".to_string(),
    })
}