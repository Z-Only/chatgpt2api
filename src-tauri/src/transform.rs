use serde_json::{json, Value};

use crate::{
    config::AppConfig,
    error::{AppError, AppResult},
    models::ModelRegistry,
    tools::convert_openai_tools,
    upstream::ImageGenerationRequest,
};

pub fn chat_to_responses(request: &Value, config: &AppConfig) -> AppResult<Value> {
    let messages = request
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_request("messages must be an array"))?;
    let model = request.get("model").and_then(Value::as_str);
    let resolved = ModelRegistry::from_config(config).resolve(model, config)?;
    let effort = request
        .pointer("/reasoning/effort")
        .and_then(Value::as_str)
        .unwrap_or(&resolved.reasoning_effort);
    let summary = request
        .pointer("/reasoning/summary")
        .and_then(Value::as_str)
        .or(config.reasoning.summary.as_deref());
    let verbosity = request
        .pointer("/text/verbosity")
        .and_then(Value::as_str)
        .unwrap_or(&config.text.verbosity);

    let mut response = json!({
        "model": resolved.model,
        "input": convert_messages(messages)?,
        "reasoning": {"effort": effort},
        "text": {"verbosity": verbosity},
        "stream": true,
        "store": false,
    });
    if let Some(summary) = summary {
        response["reasoning"]["summary"] = json!(summary);
    }
    if let Some(tools) = request.get("tools").and_then(Value::as_array) {
        response["tools"] = json!(convert_openai_tools(tools));
    }

    Ok(response)
}

pub fn image_generation_to_upstream(
    request: &Value,
    config: &AppConfig,
) -> AppResult<ImageGenerationRequest> {
    Ok(ImageGenerationRequest {
        prompt: string_field(request, "prompt")?.to_string(),
        model: optional_string(request, "model")
            .unwrap_or(&config.image.default_model)
            .to_string(),
        size: Some(
            optional_string(request, "size")
                .unwrap_or(&config.image.size)
                .to_string(),
        ),
        quality: Some(
            optional_string(request, "quality")
                .unwrap_or(&config.image.quality)
                .to_string(),
        ),
        background: Some(
            optional_string(request, "background")
                .unwrap_or(&config.image.background)
                .to_string(),
        ),
        output_format: Some(
            optional_string(request, "output_format")
                .unwrap_or(&config.image.output_format)
                .to_string(),
        ),
        output_compression: request
            .get("output_compression")
            .and_then(Value::as_u64)
            .map(|value| value as u8)
            .or(config.image.output_compression),
    })
}

pub fn responses_image_output_to_openai_image_response(response: &Value) -> AppResult<Value> {
    let output = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_request("output must be an array"))?;
    let data: Vec<Value> = output
        .iter()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("image_generation_call"))
        .filter_map(|item| item.get("result").and_then(Value::as_str))
        .map(|b64_json| json!({ "b64_json": b64_json }))
        .collect();

    if data.is_empty() {
        return Err(invalid_request(
            "response output did not include generated image data",
        ));
    }

    Ok(json!({ "created": 0, "data": data }))
}

fn convert_messages(messages: &[Value]) -> AppResult<Vec<Value>> {
    messages
        .iter()
        .map(|message| {
            let role = string_field(message, "role")?;
            Ok(json!({
                "role": role,
                "content": convert_content(role, message.get("content").ok_or_else(|| invalid_request("message content is required"))?)?,
            }))
        })
        .collect()
}

fn convert_content(role: &str, content: &Value) -> AppResult<Vec<Value>> {
    let text_type = if role == "assistant" {
        "output_text"
    } else {
        "input_text"
    };
    if let Some(text) = content.as_str() {
        return Ok(vec![json!({"type": text_type, "text": text})]);
    }

    let parts = content
        .as_array()
        .ok_or_else(|| invalid_request("message content must be a string or array"))?;
    parts
        .iter()
        .map(|part| match part.get("type").and_then(Value::as_str) {
            Some("text") => Ok(json!({
                "type": text_type,
                "text": string_field(part, "text")?,
            })),
            Some("image_url") => Ok(json!({
                "type": "input_image",
                "image_url": part
                    .pointer("/image_url/url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid_request("image_url.url is required"))?,
            })),
            _ => Err(invalid_request("unsupported message content part")),
        })
        .collect()
}

fn string_field<'a>(value: &'a Value, field: &str) -> AppResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_request(&format!("{field} must be a string")))
}

fn optional_string<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn invalid_request(message: &str) -> AppError {
    AppError::InvalidRequest(message.to_string())
}
