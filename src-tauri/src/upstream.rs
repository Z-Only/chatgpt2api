use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use reqwest::{multipart, StatusCode, Url};
use serde::Serialize;
use serde_json::Value;

use crate::{
    error::{AppError, AppResult},
    rate_limits::{parse_usage_limit_headers, UsageLimit},
    sse::responses_sse_to_response_json,
};

type RefreshFn =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = AppResult<String>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct UpstreamClient {
    http: reqwest::Client,
    base_url: Url,
    access_token: Arc<RwLock<String>>,
    session_id: String,
    installation_id: String,
    refresh: Option<RefreshFn>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageEditRequest {
    pub prompt: String,
    pub model: String,
    pub image: Vec<u8>,
    pub image_filename: String,
    pub mask: Option<Vec<u8>>,
    pub mask_filename: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub background: Option<String>,
    pub output_format: Option<String>,
    pub output_compression: Option<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpstreamJsonResponse {
    pub body: Value,
    pub rate_limits: Vec<UsageLimit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageResponse {
    pub images: Vec<ImageOutput>,
    pub rate_limits: Vec<UsageLimit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageOutput {
    pub b64_json: String,
}

impl UpstreamClient {
    pub fn new(
        base_url: impl AsRef<str>,
        access_token: impl Into<String>,
        session_id: impl Into<String>,
        installation_id: impl Into<String>,
    ) -> AppResult<Self> {
        let base_url = normalize_base_url(base_url.as_ref())?;

        Ok(Self {
            http: reqwest::Client::new(),
            base_url,
            access_token: Arc::new(RwLock::new(access_token.into())),
            session_id: session_id.into(),
            installation_id: installation_id.into(),
            refresh: None,
        })
    }

    pub fn with_refresh<F, Fut>(mut self, refresh: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AppResult<String>> + Send + 'static,
    {
        self.refresh = Some(Arc::new(move || Box::pin(refresh())));
        self
    }

    pub async fn send_responses(&self, payload: &Value) -> AppResult<UpstreamJsonResponse> {
        let url = self.endpoint_url("responses")?;
        let response = self
            .send_with_auth(|token| {
                self.authed(self.http.post(url.clone()), token)
                    .json(payload)
            })
            .await?;

        self.read_json_response(response).await
    }

    pub async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> AppResult<ImageResponse> {
        let url = self.endpoint_url("images/generations")?;
        let response = self
            .send_with_auth(|token| {
                self.authed(self.http.post(url.clone()), token)
                    .json(&request)
            })
            .await?;

        self.read_image_response(response).await
    }

    pub async fn edit_image(&self, request: ImageEditRequest) -> AppResult<ImageResponse> {
        let url = self.endpoint_url("images/edits")?;
        let response = self
            .send_with_auth(|token| {
                self.authed(self.http.post(url.clone()), token)
                    .multipart(request.multipart_form())
            })
            .await?;

        self.read_image_response(response).await
    }

    async fn send_with_auth<F>(&self, build: F) -> AppResult<reqwest::Response>
    where
        F: Fn(&str) -> reqwest::RequestBuilder,
    {
        let access_token = self.access_token();
        let response = build(&access_token).send().await?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }

        let Some(refresh) = &self.refresh else {
            return Ok(response);
        };
        let access_token = refresh().await?;
        *self
            .access_token
            .write()
            .expect("access token lock poisoned") = access_token.clone();

        Ok(build(&access_token).send().await?)
    }

    async fn read_json_response(
        &self,
        response: reqwest::Response,
    ) -> AppResult<UpstreamJsonResponse> {
        let response = ensure_success(response).await?;
        let rate_limits = parse_usage_limit_headers(response.headers());
        let is_sse = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream"));
        let text = response.text().await?;
        let body = if is_sse || text.lines().any(|line| line.trim().starts_with("data:")) {
            responses_sse_to_response_json(&text)?
        } else {
            serde_json::from_str(&text).map_err(|error| {
                AppError::Upstream(format!(
                    "upstream returned non-JSON body: {error}: {}",
                    text.chars().take(500).collect::<String>()
                ))
            })?
        };

        Ok(UpstreamJsonResponse { body, rate_limits })
    }

    async fn read_image_response(&self, response: reqwest::Response) -> AppResult<ImageResponse> {
        let response = self.read_json_response(response).await?;
        let images = extract_image_outputs(&response.body);
        if images.is_empty() {
            return Err(AppError::Upstream(
                "image response did not include base64 image data".to_string(),
            ));
        }

        Ok(ImageResponse {
            images,
            rate_limits: response.rate_limits,
        })
    }

    fn authed(&self, request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
        request
            .bearer_auth(token)
            .header("session-id", &self.session_id)
            .header("oai-device-id", &self.installation_id)
    }

    fn access_token(&self) -> String {
        self.access_token
            .read()
            .expect("access token lock poisoned")
            .clone()
    }

    fn endpoint_url(&self, path: &str) -> AppResult<Url> {
        self.base_url
            .join(path)
            .map_err(|error| AppError::Upstream(format!("invalid upstream URL: {error}")))
    }
}

impl ImageEditRequest {
    fn multipart_form(&self) -> multipart::Form {
        let image =
            multipart::Part::bytes(self.image.clone()).file_name(self.image_filename.clone());
        let mut form = multipart::Form::new()
            .text("prompt", self.prompt.clone())
            .text("model", self.model.clone())
            .part("image", image);

        if let Some(mask) = &self.mask {
            form = form.part(
                "mask",
                multipart::Part::bytes(mask.clone()).file_name(
                    self.mask_filename
                        .clone()
                        .unwrap_or_else(|| "mask.png".to_string()),
                ),
            );
        }
        if let Some(size) = &self.size {
            form = form.text("size", size.clone());
        }
        if let Some(quality) = &self.quality {
            form = form.text("quality", quality.clone());
        }
        if let Some(background) = &self.background {
            form = form.text("background", background.clone());
        }
        if let Some(output_format) = &self.output_format {
            form = form.text("output_format", output_format.clone());
        }
        if let Some(output_compression) = self.output_compression {
            form = form.text("output_compression", output_compression.to_string());
        }

        form
    }
}

fn normalize_base_url(base_url: &str) -> AppResult<Url> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::Upstream(
            "upstream base URL must not be empty".to_string(),
        ));
    }

    let normalized = format!("{}/", trimmed.trim_end_matches('/'));
    Url::parse(&normalized)
        .map_err(|error| AppError::Upstream(format!("invalid upstream base URL: {error}")))
}

async fn ensure_success(response: reqwest::Response) -> AppResult<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    let body = body.chars().take(500).collect::<String>();
    Err(AppError::Upstream(format!(
        "upstream returned {status}: {body}"
    )))
}

fn extract_image_outputs(body: &Value) -> Vec<ImageOutput> {
    let mut images = Vec::new();

    if let Some(data) = body.get("data").and_then(Value::as_array) {
        images.extend(data.iter().filter_map(|item| {
            item.get("b64_json")
                .and_then(Value::as_str)
                .map(|b64_json| ImageOutput {
                    b64_json: b64_json.to_string(),
                })
        }));
    }

    if let Some(output) = body.get("output").and_then(Value::as_array) {
        images.extend(output.iter().filter_map(|item| {
            item.get("result")
                .and_then(Value::as_str)
                .map(|b64_json| ImageOutput {
                    b64_json: b64_json.to_string(),
                })
        }));
    }

    images
}
