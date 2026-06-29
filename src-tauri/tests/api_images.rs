use axum::{
    body::Bytes,
    extract::{OriginalUri, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chatgpt2api::{api::ApiState, config::AppConfig, server, upstream::UpstreamClient};
use serde_json::{json, Value};
use tokio::sync::mpsc;

#[derive(Debug)]
struct RecordedRequest {
    path: String,
    body: Vec<u8>,
}

#[derive(Clone)]
struct FakeUpstream {
    tx: mpsc::UnboundedSender<RecordedRequest>,
}

async fn fake_image(
    OriginalUri(uri): OriginalUri,
    State(state): State<FakeUpstream>,
    body: Bytes,
) -> impl IntoResponse {
    state
        .tx
        .send(RecordedRequest {
            path: uri.path().to_string(),
            body: body.to_vec(),
        })
        .unwrap();
    (
        StatusCode::OK,
        Json(json!({"data": [{"b64_json": "base64-image"}]})),
    )
}

async fn spawn_fake_upstream() -> (String, mpsc::UnboundedReceiver<RecordedRequest>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let app = Router::new()
        .route("/images/generations", post(fake_image))
        .route("/images/edits", post(fake_image))
        .with_state(FakeUpstream { tx });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://{addr}"), rx)
}

async fn spawn_api(
    mut config: AppConfig,
) -> (
    server::ServerHandle,
    mpsc::UnboundedReceiver<RecordedRequest>,
) {
    config.server.port = free_port().await;
    let (base_url, rx) = spawn_fake_upstream().await;
    let upstream = UpstreamClient::new(base_url, "token", "session", "install").unwrap();
    let handle = server::spawn_with_state(ApiState::with_upstream(config, upstream))
        .await
        .unwrap();

    (handle, rx)
}

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::test]
async fn api_image_generation_returns_b64_json_and_applies_defaults() {
    let mut config = AppConfig::default();
    config.image.size = "1024x1536".to_string();
    let (handle, mut upstream_requests) = spawn_api(config).await;
    let response: Value = reqwest::Client::new()
        .post(format!("http://{}/v1/images/generations", handle.addr()))
        .json(&json!({"prompt": "draw"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let upstream_request = upstream_requests.recv().await.unwrap();
    let upstream_body: Value = serde_json::from_slice(&upstream_request.body).unwrap();

    assert_eq!(response["data"][0]["b64_json"], "base64-image");
    assert_eq!(upstream_request.path, "/images/generations");
    assert_eq!(upstream_body["model"], "chatgpt-image-latest");
    assert_eq!(upstream_body["size"], "1024x1536");
    handle.stop();
}

#[tokio::test]
async fn api_image_edit_accepts_multipart_upload() {
    let (handle, mut upstream_requests) = spawn_api(AppConfig::default()).await;
    let form = reqwest::multipart::Form::new()
        .text("prompt", "edit this")
        .part(
            "image",
            reqwest::multipart::Part::bytes(b"image-bytes".to_vec()).file_name("input.png"),
        );
    let response: Value = reqwest::Client::new()
        .post(format!("http://{}/v1/images/edits", handle.addr()))
        .multipart(form)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let upstream_request = upstream_requests.recv().await.unwrap();
    let body = String::from_utf8_lossy(&upstream_request.body);

    assert_eq!(response["data"][0]["b64_json"], "base64-image");
    assert_eq!(upstream_request.path, "/images/edits");
    assert!(body.contains("edit this"));
    assert!(body.contains("image-bytes"));
    handle.stop();
}

#[tokio::test]
async fn api_image_variation_returns_openai_shaped_501() {
    let (handle, _rx) = spawn_api(AppConfig::default()).await;
    let response = reqwest::Client::new()
        .post(format!("http://{}/v1/images/variations", handle.addr()))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();

    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(body["error"]["code"], "unsupported");
    handle.stop();
}
