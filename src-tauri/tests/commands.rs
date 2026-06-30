use axum::{extract::State, routing::post, Json, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chatgpt2api::{app_state::AppState, config::AppConfig};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tokio::sync::mpsc;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    listener.local_addr().unwrap().port()
}

fn temp_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("chatgpt2api-{name}-{}-{nanos}", std::process::id()))
}

fn jwt_with_payload(payload: &str) -> String {
    format!(
        "header.{}.signature",
        URL_SAFE_NO_PAD.encode(payload.as_bytes())
    )
}

async fn spawn_fake_upstream() -> (String, mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let app = Router::new()
        .route(
            "/responses",
            post(
                |State(tx): State<mpsc::UnboundedSender<String>>,
                 headers: axum::http::HeaderMap| async move {
                    let auth = headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                        .to_string();
                    tx.send(auth).unwrap();
                    Json(json!({"id": "resp_1"}))
                },
            ),
        )
        .with_state(tx);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), rx)
}

fn write_codex_auth_file() -> PathBuf {
    let path = temp_path("codex-auth");
    let exp = chrono::Utc::now().timestamp() + 3600;
    let access_token = jwt_with_payload(&format!(r#"{{"sub":"user-123","exp":{exp}}}"#));
    let id_token = jwt_with_payload(&format!(
        r#"{{"sub":"user-123","email":"user@example.com","exp":{exp}}}"#
    ));
    fs::write(
        &path,
        format!(
            r#"{{
  "tokens": {{
    "access_token": "{access_token}",
    "refresh_token": "refresh-token",
    "id_token": "{id_token}",
    "account_id": "account-123"
  }}
}}"#
        ),
    )
    .unwrap();
    path
}

#[tokio::test]
async fn app_state_starts_and_stops_server() {
    let mut config = AppConfig::default();
    config.server.port = free_port().await;
    let state = AppState::new(config);

    let running = state.start_server().await.unwrap();
    assert!(running.running);
    assert!(running.url.ends_with(&format!(":{}", running.port)));

    state.stop_server().unwrap();
    assert!(!state.server_status().running);
}

#[tokio::test]
async fn app_state_login_credentials_enable_responses_api() {
    let (base_url, mut upstream_auth) = spawn_fake_upstream().await;
    let mut config = AppConfig::default();
    config.server.port = free_port().await;
    config.api.upstream_base_url = base_url;
    let state = AppState::new(config);
    let auth_path = write_codex_auth_file();

    let credentials = state.login_local_credentials_from_path(&auth_path).unwrap();
    assert_eq!(credentials.email.as_deref(), Some("user@example.com"));

    let running = state.start_server().await.unwrap();
    let response: Value = reqwest::Client::new()
        .post(format!("{}/v1/responses", running.url))
        .json(&json!({"input": "hi"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["id"], "resp_1");
    assert!(upstream_auth.recv().await.unwrap().starts_with("Bearer "));
}
