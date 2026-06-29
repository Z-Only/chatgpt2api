use axum::Json;
use serde_json::{json, Value};

pub async fn root() -> Json<Value> {
    Json(json!({"name": "chatgpt2api"}))
}

pub async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}
