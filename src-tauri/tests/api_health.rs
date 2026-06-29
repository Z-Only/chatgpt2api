use chatgpt2api::{config::AppConfig, server};
use reqwest::header::ORIGIN;
use serde_json::Value;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    listener.local_addr().unwrap().port()
}

async fn test_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.server.port = free_port().await;
    config
}

#[tokio::test]
async fn api_health_returns_ok() {
    let handle = server::spawn(test_config().await).await.unwrap();
    let response: Value = reqwest::get(format!("http://{}/health", handle.addr()))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(response["status"], "ok");
    handle.stop();
}

#[tokio::test]
async fn api_health_uses_custom_port() {
    let config = test_config().await;
    let expected_port = config.server.port;
    let handle = server::spawn(config).await.unwrap();

    assert_eq!(handle.addr().port(), expected_port);
    let response = reqwest::get(format!("http://{}/health", handle.addr()))
        .await
        .unwrap();
    assert!(response.status().is_success());
    handle.stop();
}

#[tokio::test]
async fn api_cors_is_local_only_and_never_wildcard() {
    let handle = server::spawn(test_config().await).await.unwrap();
    let client = reqwest::Client::new();
    let local = client
        .get(format!("http://{}/health", handle.addr()))
        .header(ORIGIN, "http://127.0.0.1:5173")
        .send()
        .await
        .unwrap();
    let external = client
        .get(format!("http://{}/health", handle.addr()))
        .header(ORIGIN, "https://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(
        local.headers()["access-control-allow-origin"],
        "http://127.0.0.1:5173"
    );
    assert_ne!(
        external.headers().get("access-control-allow-origin"),
        Some(&"*".parse().unwrap())
    );
    assert!(external
        .headers()
        .get("access-control-allow-origin")
        .is_none());
    handle.stop();
}

#[tokio::test]
async fn api_rejects_external_bind_by_default() {
    let mut config = test_config().await;
    config.server.host = "0.0.0.0".to_string();

    let error = server::spawn(config).await.unwrap_err().to_string();

    assert!(error.contains("external binds require allow_external_bind = true"));
}
