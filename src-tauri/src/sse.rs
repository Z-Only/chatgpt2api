use serde_json::{json, Value};

use crate::{config::AppConfig, error::AppResult};

pub fn responses_sse_to_response_json(input: &str) -> AppResult<Value> {
    let mut output_text = String::new();
    let mut completed = None;

    for line in input.lines() {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            continue;
        }

        let event: Value = serde_json::from_str(data)?;
        match event.get("type").and_then(Value::as_str) {
            Some("response.output_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    output_text.push_str(delta);
                }
            }
            Some("response.output_text.done") => {
                if let Some(text) = event.get("text").and_then(Value::as_str) {
                    output_text = text.to_string();
                }
            }
            Some("response.content_part.done") => {
                if let Some(text) = event.pointer("/part/text").and_then(Value::as_str) {
                    output_text.push_str(text);
                }
            }
            Some("response.output_item.done") => {
                append_item_text(&mut output_text, event.get("item"));
            }
            Some("response.completed") => {
                completed = event.get("response").cloned();
            }
            Some(_) | None => {}
        }
    }

    if let Some(response) = completed {
        if output_text.is_empty()
            || response
                .get("output")
                .and_then(Value::as_array)
                .is_some_and(|output| !output.is_empty())
        {
            return Ok(response);
        }
        let mut response = response;
        response["output"] = output_from_text(&output_text);
        return Ok(response);
    }

    Ok(json!({
        "id": "resp_stream",
        "object": "response",
        "output": output_from_text(&output_text),
    }))
}

fn output_from_text(text: &str) -> Value {
    json!([{
        "type": "message",
        "role": "assistant",
        "content": [{"type": "output_text", "text": text}],
    }])
}

fn append_item_text(output: &mut String, item: Option<&Value>) {
    let Some(content) = item
        .and_then(|item| item.get("content"))
        .and_then(Value::as_array)
    else {
        return;
    };
    for part in content {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            output.push_str(text);
        }
    }
}

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
