use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message as UpstreamMessage};

use crate::api::{openai::normalize_responses_request, ApiError, ApiState};

pub fn routes() -> Router<ApiState> {
    Router::new().route("/v1/responses", get(responses_ws))
}

async fn responses_ws(ws: WebSocketUpgrade, State(state): State<ApiState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut client: WebSocket, state: ApiState) {
    let Some(Ok(Message::Text(text))) = client.recv().await else {
        return;
    };
    let Ok(frame) = serde_json::from_str::<Value>(&text) else {
        close_client(&mut client).await;
        return;
    };
    let Ok(frame) = normalize_create_frame(frame, &state) else {
        close_client(&mut client).await;
        return;
    };
    let Some(ws_url) = state.responses_ws_url else {
        close_client(&mut client).await;
        return;
    };
    let Ok((mut upstream, _)) = connect_async(ws_url).await else {
        close_client(&mut client).await;
        return;
    };
    if upstream
        .send(UpstreamMessage::Text(frame.to_string().into()))
        .await
        .is_err()
    {
        close_client(&mut client).await;
        return;
    }

    while let Some(message) = upstream.next().await {
        match message {
            Ok(UpstreamMessage::Text(text)) => {
                let terminal = is_terminal_event(&text);
                if client
                    .send(Message::Text(text.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
                if terminal {
                    close_client(&mut client).await;
                    break;
                }
            }
            Ok(UpstreamMessage::Close(_)) => {
                close_client(&mut client).await;
                break;
            }
            Ok(_) => {}
            Err(_) => {
                close_client(&mut client).await;
                break;
            }
        }
    }
}

pub fn normalize_create_frame(frame: Value, state: &ApiState) -> Result<Value, ApiError> {
    if frame.get("type").and_then(Value::as_str) != Some("response.create") {
        return Err(ApiError::bad_request(
            "first WebSocket frame must be response.create",
        ));
    }

    let response = frame
        .get("response")
        .ok_or_else(|| ApiError::bad_request("response.create requires response"))?;
    let normalized = normalize_responses_request(response, &state.config)?;
    let mut frame = frame
        .as_object()
        .cloned()
        .ok_or_else(|| ApiError::bad_request("frame must be a JSON object"))?;
    frame.insert("response".to_string(), normalized);

    Ok(Value::Object(frame))
}

fn is_terminal_event(text: &str) -> bool {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|event| {
            event.get("type").and_then(Value::as_str).map(|event_type| {
                matches!(
                    event_type,
                    "response.completed" | "response.failed" | "response.cancelled"
                )
            })
        })
        .unwrap_or(false)
}

async fn close_client(client: &mut WebSocket) {
    let _ = client.send(Message::Close(None)).await;
}
