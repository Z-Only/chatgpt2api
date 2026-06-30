use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Map, Value};

use crate::{
    api::{ApiError, ApiState},
    image::image_model_list,
    models::ModelRegistry,
    transform::chat_to_responses,
};

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/models", get(models))
        .route("/v1/responses", post(responses))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/completions", post(completions))
}

async fn models(State(state): State<ApiState>) -> Json<Value> {
    let mut models = ModelRegistry::from_config(&state.config).public_models();
    models.extend(image_model_list());
    Json(json!({
        "object": "list",
        "data": models.into_iter().map(|id| json!({
            "id": id,
            "object": "model",
            "owned_by": "chatgpt2api",
        })).collect::<Vec<_>>()
    }))
}

async fn responses(
    State(state): State<ApiState>,
    Json(request): Json<Value>,
) -> Result<Response, ApiError> {
    let upstream = state.upstream()?;
    let payload = normalize_responses_request(&request, &state.config)?;
    let response = upstream.send_responses(&payload).await?;

    Ok(Json(response.body).into_response())
}

async fn chat_completions(
    State(state): State<ApiState>,
    Json(request): Json<Value>,
) -> Result<Response, ApiError> {
    let upstream = state.upstream()?;
    let stream = request
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let payload = chat_to_responses(&request, &state.config)?;
    let response = upstream.send_responses(&payload).await?;

    if stream {
        return Ok((
            [("content-type", "text/event-stream")],
            chat_sse_from_response(&response.body)?,
        )
            .into_response());
    }

    Ok(Json(chat_completion_from_response(
        &response.body,
        payload
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("gpt-5.5"),
    ))
    .into_response())
}

async fn completions(
    State(state): State<ApiState>,
    Json(request): Json<Value>,
) -> Result<Response, ApiError> {
    let prompt = request.get("prompt").and_then(Value::as_str).unwrap_or("");
    let mut chat_request = json!({
        "messages": [{"role": "user", "content": prompt}],
    });
    if let Some(model) = request.get("model") {
        chat_request["model"] = model.clone();
    }

    let upstream = state.upstream()?;
    let payload = chat_to_responses(&chat_request, &state.config)?;
    let response = upstream.send_responses(&payload).await?;
    let text = collect_output_text(&response.body);

    Ok(Json(json!({
        "id": response.body.get("id").cloned().unwrap_or_else(|| json!("cmpl_local")),
        "object": "text_completion",
        "choices": [{"index": 0, "text": text, "finish_reason": "stop"}],
    }))
    .into_response())
}

pub fn normalize_responses_request(
    request: &Value,
    config: &crate::config::AppConfig,
) -> Result<Value, ApiError> {
    let mut object = request
        .as_object()
        .cloned()
        .ok_or_else(|| ApiError::bad_request("request must be a JSON object"))?;
    let model = request.get("model").and_then(Value::as_str);
    let resolved = ModelRegistry::from_config(config).resolve(model, config)?;

    object.insert("model".to_string(), json!(resolved.model));
    merge_object(
        &mut object,
        "reasoning",
        "effort",
        json!(resolved.reasoning_effort),
    );
    if let Some(summary) = &config.reasoning.summary {
        merge_object(&mut object, "reasoning", "summary", json!(summary));
    }
    merge_object(
        &mut object,
        "text",
        "verbosity",
        json!(config.text.verbosity),
    );
    if let Some(input) = object.get("input").cloned().filter(Value::is_string) {
        object.insert(
            "input".to_string(),
            json!([{
                "role": "user",
                "content": [{"type": "input_text", "text": input.as_str().unwrap_or_default()}],
            }]),
        );
    }
    object.insert("store".to_string(), json!(false));
    object.insert("stream".to_string(), json!(true));

    Ok(Value::Object(object))
}

fn merge_object(object: &mut Map<String, Value>, section: &str, key: &str, default: Value) {
    let section_value = object
        .entry(section.to_string())
        .or_insert_with(|| json!({}));
    if let Some(section_object) = section_value.as_object_mut() {
        section_object.entry(key.to_string()).or_insert(default);
    }
}

fn chat_completion_from_response(response: &Value, model: &str) -> Value {
    let text = collect_output_text(response);
    let mut message = json!({
        "role": "assistant",
        "content": text,
    });
    let tool_calls = collect_tool_calls(response);
    if !tool_calls.is_empty() {
        message["tool_calls"] = json!(tool_calls);
    }

    json!({
        "id": response.get("id").cloned().unwrap_or_else(|| json!("chatcmpl_local")),
        "object": "chat.completion",
        "created": 0,
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": "stop",
        }],
        "usage": openai_usage(response.get("usage")),
    })
}

fn chat_sse_from_response(response: &Value) -> Result<String, ApiError> {
    let text = collect_output_text(response);
    Ok(format!(
        "data: {}\n\ndata: [DONE]\n\n",
        serde_json::to_string(&json!({
            "choices": [{
                "index": 0,
                "delta": {"content": text},
            }]
        }))
        .map_err(|error| ApiError::bad_request(error.to_string()))?
    ))
}

fn collect_output_text(response: &Value) -> String {
    response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|content| {
            content
                .get("text")
                .or_else(|| content.get("delta"))
                .and_then(Value::as_str)
        })
        .collect::<Vec<_>>()
        .join("")
}

fn collect_tool_calls(response: &Value) -> Vec<Value> {
    response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
        .map(|item| {
            json!({
                "id": item.get("call_id").or_else(|| item.get("id")).cloned().unwrap_or_else(|| json!("call_local")),
                "type": "function",
                "function": {
                    "name": item.get("name").cloned().unwrap_or_else(|| json!("")),
                    "arguments": item.get("arguments").cloned().unwrap_or_else(|| json!("{}")),
                }
            })
        })
        .collect()
}

fn openai_usage(usage: Option<&Value>) -> Value {
    let Some(usage) = usage else {
        return json!({});
    };

    json!({
        "prompt_tokens": usage.get("input_tokens").cloned().unwrap_or_else(|| json!(0)),
        "completion_tokens": usage.get("output_tokens").cloned().unwrap_or_else(|| json!(0)),
        "total_tokens": usage.get("total_tokens").cloned().unwrap_or_else(|| json!(0)),
    })
}
