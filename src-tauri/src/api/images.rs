use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::Value;

use crate::{
    api::{ApiError, ApiState},
    image::openai_image_response,
    transform::image_generation_to_upstream,
    upstream::ImageEditRequest,
};

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/v1/images/generations", post(generations))
        .route("/v1/images/edits", post(edits))
        .route("/v1/images/variations", post(variations))
}

async fn generations(
    State(state): State<ApiState>,
    Json(request): Json<Value>,
) -> Result<Response, ApiError> {
    let upstream = state.upstream()?;
    let request = image_generation_to_upstream(&request, &state.config)?;
    let response = upstream.generate_image(request).await?;

    Ok(Json(openai_image_response(response)).into_response())
}

async fn edits(State(state): State<ApiState>, multipart: Multipart) -> Result<Response, ApiError> {
    let upstream = state.upstream()?;
    let request = image_edit_to_upstream(multipart, &state.config).await?;
    let response = upstream.edit_image(request).await?;

    Ok(Json(openai_image_response(response)).into_response())
}

async fn variations() -> ApiError {
    ApiError::unsupported("image variations are not supported by the upstream image path")
}

async fn image_edit_to_upstream(
    mut multipart: Multipart,
    config: &crate::config::AppConfig,
) -> Result<ImageEditRequest, ApiError> {
    let mut prompt = None;
    let mut model = None;
    let mut image = None;
    let mut image_filename = None;
    let mut mask = None;
    let mut mask_filename = None;
    let mut size = None;
    let mut quality = None;
    let mut background = None;
    let mut output_format = None;
    let mut output_compression = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        let filename = field.file_name().map(str::to_string);

        match name.as_str() {
            "image" => {
                image_filename = filename;
                image = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?
                        .to_vec(),
                );
            }
            "mask" => {
                mask_filename = filename;
                mask = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|error| ApiError::bad_request(error.to_string()))?
                        .to_vec(),
                );
            }
            "prompt" => prompt = Some(field_text(field).await?),
            "model" => model = Some(field_text(field).await?),
            "size" => size = Some(field_text(field).await?),
            "quality" => quality = Some(field_text(field).await?),
            "background" => background = Some(field_text(field).await?),
            "output_format" => output_format = Some(field_text(field).await?),
            "output_compression" => {
                output_compression =
                    Some(field_text(field).await?.parse().map_err(|_| {
                        ApiError::bad_request("output_compression must be 0..=100")
                    })?);
            }
            _ => {}
        }
    }

    Ok(ImageEditRequest {
        prompt: prompt.ok_or_else(|| ApiError::bad_request("prompt is required"))?,
        model: model.unwrap_or_else(|| config.image.default_model.clone()),
        image: image.ok_or_else(|| ApiError::bad_request("image is required"))?,
        image_filename: image_filename.unwrap_or_else(|| "image.png".to_string()),
        mask,
        mask_filename,
        size: Some(size.unwrap_or_else(|| config.image.size.clone())),
        quality: Some(quality.unwrap_or_else(|| config.image.quality.clone())),
        background: Some(background.unwrap_or_else(|| config.image.background.clone())),
        output_format: Some(output_format.unwrap_or_else(|| config.image.output_format.clone())),
        output_compression: output_compression.or(config.image.output_compression),
    })
}

async fn field_text(field: axum::extract::multipart::Field<'_>) -> Result<String, ApiError> {
    field
        .text()
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))
}
