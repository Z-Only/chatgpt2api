use chatgpt2api::{api::ApiState, config::AppConfig, server};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    listener.local_addr().unwrap().port()
}

async fn spawn_fake_upstream_ws() -> (String, mpsc::UnboundedReceiver<Value>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        if let Some(Ok(Message::Text(text))) = socket.next().await {
            tx.send(serde_json::from_str(&text).unwrap()).unwrap();
            socket
                .send(Message::Text(
                    json!({"type": "response.completed"}).to_string().into(),
                ))
                .await
                .unwrap();
        }
    });

    (format!("ws://{addr}"), rx)
}

async fn spawn_api(ws_url: String) -> server::ServerHandle {
    let mut config = AppConfig::default();
    config.server.port = free_port().await;

    server::spawn_with_state(ApiState::new(config).with_responses_ws_url(ws_url))
        .await
        .unwrap()
}

#[tokio::test]
async fn responses_ws_invalid_first_frame_closes() {
    let (ws_url, _rx) = spawn_fake_upstream_ws().await;
    let handle = spawn_api(ws_url).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{}/v1/responses", handle.addr()))
            .await
            .unwrap();

    socket
        .send(Message::Text(json!({"type": "bad"}).to_string().into()))
        .await
        .unwrap();
    let next = socket.next().await;

    assert!(matches!(next, None | Some(Ok(Message::Close(_)))));
    handle.stop();
}

#[tokio::test]
async fn responses_ws_normalized_create_frame_forwards() {
    let (ws_url, mut upstream_messages) = spawn_fake_upstream_ws().await;
    let handle = spawn_api(ws_url).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{}/v1/responses", handle.addr()))
            .await
            .unwrap();

    socket
        .send(Message::Text(
            json!({"type": "response.create", "response": {"input": "hi"}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let upstream = upstream_messages.recv().await.unwrap();

    assert_eq!(upstream["type"], "response.create");
    assert_eq!(upstream["response"]["model"], "gpt-5.5");
    assert_eq!(upstream["response"]["reasoning"]["effort"], "medium");
    assert_eq!(upstream["response"]["text"]["verbosity"], "medium");
    handle.stop();
}

#[tokio::test]
async fn responses_ws_terminal_event_closes_local_stream() {
    let (ws_url, _rx) = spawn_fake_upstream_ws().await;
    let handle = spawn_api(ws_url).await;
    let (mut socket, _) =
        tokio_tungstenite::connect_async(format!("ws://{}/v1/responses", handle.addr()))
            .await
            .unwrap();

    socket
        .send(Message::Text(
            json!({"type": "response.create", "response": {"input": "hi"}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let terminal = socket.next().await.unwrap().unwrap();
    let after_terminal = socket.next().await;

    assert!(terminal.to_text().unwrap().contains("response.completed"));
    assert!(matches!(after_terminal, None | Some(Ok(Message::Close(_)))));
    handle.stop();
}
