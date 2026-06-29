use serde_json::{json, Value};

use crate::upstream::ImageResponse;

pub const IMAGE_MODELS: &[&str] = &["chatgpt-image-latest", "gpt-image-1"];

pub fn image_model_list() -> Vec<String> {
    IMAGE_MODELS
        .iter()
        .map(|model| (*model).to_string())
        .collect()
}

pub fn openai_image_response(response: ImageResponse) -> Value {
    let data: Vec<Value> = response
        .images
        .into_iter()
        .map(|image| json!({ "b64_json": image.b64_json }))
        .collect();

    json!({ "created": 0, "data": data })
}
