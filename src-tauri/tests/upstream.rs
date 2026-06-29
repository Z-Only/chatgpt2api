use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Bytes,
    extract::{OriginalUri, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use chatgpt2api::{
    rate_limits::parse_usage_limit_headers,
    upstream::{ImageEditRequest, ImageGenerationRequest, UpstreamClient},
};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
struct RecordedRequest {
    path: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

#[derive(Clone)]
struct CaptureState {
    tx: mpsc::UnboundedSender<RecordedRequest>,
    statuses: Arc<Mutex<Vec<StatusCode>>>,
    response_body: Value,
    response_headers: Vec<(HeaderName, HeaderValue)>,
}

async fn capture_handler(
    OriginalUri(uri): OriginalUri,
    State(state): State<CaptureState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    state
        .tx
        .send(RecordedRequest {
            path: uri.path().to_string(),
            headers,
            body: body.to_vec(),
        })
        .unwrap();

    let status = {
        let mut statuses = state.statuses.lock().await;
        if statuses.len() > 1 {
            statuses.remove(0)
        } else {
            statuses[0]
        }
    };

    let mut response = Json(state.response_body.clone()).into_response();
    *response.status_mut() = status;
    for (name, value) in &state.response_headers {
        response.headers_mut().insert(name.clone(), value.clone());
    }
    response
}

async fn spawn_capture_server(
    statuses: Vec<StatusCode>,
    response_body: Value,
    response_headers: Vec<(&'static str, &'static str)>,
) -> (String, mpsc::UnboundedReceiver<RecordedRequest>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let state = CaptureState {
        tx,
        statuses: Arc::new(Mutex::new(statuses)),
        response_body,
        response_headers: response_headers
            .into_iter()
            .map(|(name, value)| {
                (
                    HeaderName::from_static(name),
                    HeaderValue::from_static(value),
                )
            })
            .collect(),
    };
    let app = Router::new()
        .route("/responses", post(capture_handler))
        .route("/images/generations", post(capture_handler))
        .route("/images/edits", post(capture_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://{addr}"), rx)
}

#[tokio::test]
async fn upstream_responses_send_auth_session_and_installation_headers() {
    let (base_url, mut requests) = spawn_capture_server(
        vec![StatusCode::OK],
        json!({"ok": true}),
        vec![
            ("x-ratelimit-limit-requests", "100"),
            ("x-ratelimit-remaining-requests", "42"),
        ],
    )
    .await;
    let client =
        UpstreamClient::new(base_url, "access-token", "session-123", "install-456").unwrap();

    let response = client
        .send_responses(&json!({"model": "gpt-5.5", "input": "hello"}))
        .await
        .unwrap();
    let request = requests.recv().await.unwrap();

    assert_eq!(request.path, "/responses");
    assert_eq!(request.headers["authorization"], "Bearer access-token");
    assert_eq!(request.headers["session-id"], "session-123");
    assert_eq!(request.headers["oai-device-id"], "install-456");
    assert_eq!(response.body, json!({"ok": true}));
    assert_eq!(response.rate_limits[0].name, "requests");
    assert_eq!(response.rate_limits[0].limit, Some(100));
    assert_eq!(response.rate_limits[0].remaining, Some(42));
}

#[tokio::test]
async fn upstream_unauthorized_response_refreshes_and_retries_once() {
    let (base_url, mut requests) = spawn_capture_server(
        vec![StatusCode::UNAUTHORIZED, StatusCode::OK],
        json!({"ok": true}),
        vec![],
    )
    .await;
    let refreshes = Arc::new(AtomicUsize::new(0));
    let refreshes_for_client = Arc::clone(&refreshes);
    let client = UpstreamClient::new(base_url, "stale-token", "session-123", "install-456")
        .unwrap()
        .with_refresh(move || {
            refreshes_for_client.fetch_add(1, Ordering::SeqCst);
            async { Ok("fresh-token".to_string()) }
        });

    client
        .send_responses(&json!({"model": "gpt-5.5", "input": "hello"}))
        .await
        .unwrap();
    let first = requests.recv().await.unwrap();
    let second = requests.recv().await.unwrap();

    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    assert_eq!(first.headers["authorization"], "Bearer stale-token");
    assert_eq!(second.headers["authorization"], "Bearer fresh-token");
    assert!(requests.try_recv().is_err());
}

#[tokio::test]
async fn upstream_image_generation_sends_prompt_and_model() {
    let (base_url, mut requests) = spawn_capture_server(
        vec![StatusCode::OK],
        json!({"data": [{"b64_json": "base64-image"}]}),
        vec![],
    )
    .await;
    let client =
        UpstreamClient::new(base_url, "access-token", "session-123", "install-456").unwrap();

    let response = client
        .generate_image(ImageGenerationRequest {
            prompt: "draw a small icon".to_string(),
            model: "chatgpt-image-latest".to_string(),
            size: Some("auto".to_string()),
            quality: None,
            background: None,
            output_format: Some("png".to_string()),
            output_compression: None,
        })
        .await
        .unwrap();
    let request = requests.recv().await.unwrap();
    let body: Value = serde_json::from_slice(&request.body).unwrap();

    assert_eq!(request.path, "/images/generations");
    assert_eq!(body["prompt"], "draw a small icon");
    assert_eq!(body["model"], "chatgpt-image-latest");
    assert_eq!(response.images[0].b64_json, "base64-image");
}

#[tokio::test]
async fn upstream_image_edit_sends_multipart_image_bytes() {
    let (base_url, mut requests) = spawn_capture_server(
        vec![StatusCode::OK],
        json!({"data": [{"b64_json": "edited-image"}]}),
        vec![],
    )
    .await;
    let client =
        UpstreamClient::new(base_url, "access-token", "session-123", "install-456").unwrap();

    let response = client
        .edit_image(ImageEditRequest {
            prompt: "edit this".to_string(),
            model: "chatgpt-image-latest".to_string(),
            image: b"image-bytes".to_vec(),
            image_filename: "input.png".to_string(),
            mask: None,
            mask_filename: None,
            size: None,
            quality: None,
            background: None,
            output_format: Some("png".to_string()),
            output_compression: None,
        })
        .await
        .unwrap();
    let request = requests.recv().await.unwrap();
    let content_type = request.headers["content-type"].to_str().unwrap();
    let body = String::from_utf8_lossy(&request.body);

    assert_eq!(request.path, "/images/edits");
    assert!(content_type.starts_with("multipart/form-data; boundary="));
    assert!(body.contains(r#"name="prompt""#));
    assert!(body.contains("edit this"));
    assert!(body.contains(r#"name="image"; filename="input.png""#));
    assert!(body.contains("image-bytes"));
    assert_eq!(response.images[0].b64_json, "edited-image");
}

#[test]
fn rate_limit_parser_ignores_malformed_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-ratelimit-limit-requests",
        "not-a-number".parse().unwrap(),
    );
    headers.insert(
        "x-ratelimit-remaining-requests",
        "also-bad".parse().unwrap(),
    );

    assert!(parse_usage_limit_headers(&headers).is_empty());
}
