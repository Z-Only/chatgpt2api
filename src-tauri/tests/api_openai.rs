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

#[derive(Clone)]
struct FakeUpstream {
    tx: mpsc::UnboundedSender<Value>,
    body: Value,
}

async fn fake_responses(
    OriginalUri(_uri): OriginalUri,
    State(state): State<FakeUpstream>,
    body: Bytes,
) -> impl IntoResponse {
    state
        .tx
        .send(serde_json::from_slice(&body).unwrap())
        .unwrap();
    (StatusCode::OK, Json(state.body))
}

async fn spawn_fake_upstream(body: Value) -> (String, mpsc::UnboundedReceiver<Value>) {
    let (tx, rx) = mpsc::unbounded_channel();
    let app = Router::new()
        .route("/responses", post(fake_responses))
        .with_state(FakeUpstream { tx, body });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    (format!("http://{addr}"), rx)
}

async fn spawn_api(
    mut config: AppConfig,
    upstream_body: Value,
) -> (server::ServerHandle, mpsc::UnboundedReceiver<Value>) {
    config.server.port = free_port().await;
    let (base_url, rx) = spawn_fake_upstream(upstream_body).await;
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
async fn api_models_include_default_text_and_image_models() {
    let (handle, _rx) = spawn_api(AppConfig::default(), json!({})).await;
    let response: Value = reqwest::get(format!("http://{}/v1/models", handle.addr()))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(response["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["id"] == "gpt-5.5"));
    assert!(response["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|model| model["id"] == "chatgpt-image-latest"));
    handle.stop();
}

#[tokio::test]
async fn api_chat_completion_non_streaming_returns_openai_shape() {
    let upstream_body = json!({
        "id": "resp_1",
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": "hello"}]
        }],
        "usage": {"input_tokens": 1, "output_tokens": 2, "total_tokens": 3}
    });
    let (handle, mut upstream_requests) = spawn_api(AppConfig::default(), upstream_body).await;
    let response: Value = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", handle.addr()))
        .json(&json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let upstream_request = upstream_requests.recv().await.unwrap();

    assert_eq!(response["object"], "chat.completion");
    assert_eq!(response["choices"][0]["message"]["role"], "assistant");
    assert_eq!(response["choices"][0]["message"]["content"], "hello");
    assert_eq!(response["usage"]["total_tokens"], 3);
    assert_eq!(upstream_request["input"][0]["content"][0]["text"], "hi");
    handle.stop();
}

#[tokio::test]
async fn api_chat_completion_streaming_emits_sse() {
    let upstream_body = json!({
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": "hello"}]
        }]
    });
    let (handle, _rx) = spawn_api(AppConfig::default(), upstream_body).await;
    let body = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", handle.addr()))
        .json(&json!({
            "model": "gpt-5.5",
            "stream": true,
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(body.contains("data: "));
    assert!(body.contains(r#""content":"hello""#));
    assert!(body.contains("data: [DONE]"));
    handle.stop();
}

#[tokio::test]
async fn api_responses_non_streaming_forwards_normalized_request() {
    let mut config = AppConfig::default();
    config.api.default_model = "configured-model".to_string();
    let (handle, mut upstream_requests) = spawn_api(config, json!({"id": "resp_1"})).await;
    let response: Value = reqwest::Client::new()
        .post(format!("http://{}/v1/responses", handle.addr()))
        .json(&json!({"input": "hi"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let upstream_request = upstream_requests.recv().await.unwrap();

    assert_eq!(response["id"], "resp_1");
    assert_eq!(upstream_request["model"], "configured-model");
    assert_eq!(upstream_request["input"][0]["role"], "user");
    assert_eq!(upstream_request["input"][0]["content"][0]["text"], "hi");
    assert_eq!(upstream_request["store"], false);
    assert_eq!(upstream_request["stream"], true);
    assert_eq!(upstream_request["reasoning"]["effort"], "medium");
    assert_eq!(upstream_request["text"]["verbosity"], "medium");
    handle.stop();
}

#[tokio::test]
async fn api_invalid_json_returns_400() {
    let (handle, _rx) = spawn_api(AppConfig::default(), json!({})).await;
    let response = reqwest::Client::new()
        .post(format!("http://{}/v1/chat/completions", handle.addr()))
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    handle.stop();
}
