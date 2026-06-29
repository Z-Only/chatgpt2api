use chatgpt2api::{
    config::AppConfig,
    transform::{
        chat_to_responses, image_generation_to_upstream,
        responses_image_output_to_openai_image_response,
    },
};
use serde_json::json;

#[test]
fn transform_converts_chat_image_input_to_responses_input() {
    let response = chat_to_responses(
        &json!({
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "what is this?"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc"}}
                ]
            }]
        }),
        &AppConfig::default(),
    )
    .unwrap();

    assert_eq!(response["input"][0]["content"][0]["type"], "input_text");
    assert_eq!(response["input"][0]["content"][1]["type"], "input_image");
    assert_eq!(
        response["input"][0]["content"][1]["image_url"],
        "data:image/png;base64,abc"
    );
}

#[test]
fn transform_maps_generated_image_output_to_openai_image_response() {
    let response = responses_image_output_to_openai_image_response(&json!({
        "output": [{"type": "image_generation_call", "result": "base64-image"}]
    }))
    .unwrap();

    assert_eq!(response["data"][0]["b64_json"], "base64-image");
}

#[test]
fn transform_converts_openai_function_tools_to_responses_tools() {
    let response = chat_to_responses(
        &json!({
            "messages": [{"role": "user", "content": "use a tool"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "lookup",
                    "description": "Look up a value",
                    "parameters": {"type": "object"}
                }
            }]
        }),
        &AppConfig::default(),
    )
    .unwrap();

    assert_eq!(response["tools"][0]["type"], "function");
    assert_eq!(response["tools"][0]["name"], "lookup");
    assert_eq!(response["tools"][0]["description"], "Look up a value");
    assert_eq!(response["tools"][0]["parameters"]["type"], "object");
}

#[test]
fn transform_uses_config_defaults_when_request_omits_fields() {
    let mut config = AppConfig::default();
    config.api.default_model = "configured-model".to_string();
    config.reasoning.effort = "high".to_string();
    config.reasoning.summary = Some("concise".to_string());
    config.text.verbosity = "low".to_string();
    config.image.default_model = "configured-image".to_string();
    config.image.size = "1024x1536".to_string();
    config.image.quality = "high".to_string();
    config.image.background = "transparent".to_string();
    config.image.output_format = "webp".to_string();
    config.image.output_compression = Some(80);

    let text = chat_to_responses(
        &json!({"messages": [{"role": "user", "content": "hello"}]}),
        &config,
    )
    .unwrap();
    let image = image_generation_to_upstream(&json!({"prompt": "draw"}), &config).unwrap();

    assert_eq!(text["model"], "configured-model");
    assert_eq!(text["reasoning"]["effort"], "high");
    assert_eq!(text["reasoning"]["summary"], "concise");
    assert_eq!(text["text"]["verbosity"], "low");
    assert_eq!(image.model, "configured-image");
    assert_eq!(image.size.as_deref(), Some("1024x1536"));
    assert_eq!(image.quality.as_deref(), Some("high"));
    assert_eq!(image.background.as_deref(), Some("transparent"));
    assert_eq!(image.output_format.as_deref(), Some("webp"));
    assert_eq!(image.output_compression, Some(80));
}

#[test]
fn transform_request_fields_override_config_defaults() {
    let mut config = AppConfig::default();
    config.api.default_model = "configured-model".to_string();
    config.reasoning.effort = "low".to_string();
    config.reasoning.summary = Some("auto".to_string());
    config.text.verbosity = "low".to_string();
    config.image.default_model = "configured-image".to_string();

    let text = chat_to_responses(
        &json!({
            "model": "request-model",
            "messages": [{"role": "user", "content": "hello"}],
            "reasoning": {"effort": "xhigh", "summary": "detailed"},
            "text": {"verbosity": "high"}
        }),
        &config,
    )
    .unwrap();
    let image = image_generation_to_upstream(
        &json!({
            "prompt": "draw",
            "model": "request-image",
            "size": "1536x1024",
            "quality": "medium",
            "background": "opaque",
            "output_format": "jpeg",
            "output_compression": 70
        }),
        &config,
    )
    .unwrap();

    assert_eq!(text["model"], "request-model");
    assert_eq!(text["reasoning"]["effort"], "xhigh");
    assert_eq!(text["reasoning"]["summary"], "detailed");
    assert_eq!(text["text"]["verbosity"], "high");
    assert_eq!(image.model, "request-image");
    assert_eq!(image.size.as_deref(), Some("1536x1024"));
    assert_eq!(image.quality.as_deref(), Some("medium"));
    assert_eq!(image.background.as_deref(), Some("opaque"));
    assert_eq!(image.output_format.as_deref(), Some("jpeg"));
    assert_eq!(image.output_compression, Some(70));
}
