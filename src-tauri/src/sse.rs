use serde_json::{json, Value};

use crate::{config::AppConfig, error::AppResult};

pub fn responses_sse_to_chat_sse(input: &str, config: &AppConfig) -> AppResult<Vec<String>> {
    let mut chunks = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            chunks.push(done_chunk());
            continue;
        }

        let event: Value = serde_json::from_str(data)?;
        match event.get("type").and_then(Value::as_str) {
            Some("response.output_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    chunks.push(chat_delta(delta)?);
                }
            }
            Some("response.reasoning_summary_text.delta" | "response.reasoning_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    match config.reasoning.compat.as_str() {
                        "think_tags" => {
                            chunks.push(chat_delta(&format!("<think>{delta}</think>"))?)
                        }
                        "summary" => chunks.push(chat_delta(delta)?),
                        _ => {}
                    }
                }
            }
            Some("response.completed") => chunks.push(done_chunk()),
            Some(_) | None => {}
        }
    }

    Ok(chunks)
}

fn chat_delta(content: &str) -> AppResult<String> {
    Ok(format!(
        "data: {}\n\n",
        serde_json::to_string(&json!({
            "choices": [{
                "index": 0,
                "delta": {"content": content},
            }]
        }))?
    ))
}

fn done_chunk() -> String {
    "data: [DONE]\n\n".to_string()
}
