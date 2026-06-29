use chatgpt2api::{app_state::AppState, config::AppConfig};

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    listener.local_addr().unwrap().port()
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
