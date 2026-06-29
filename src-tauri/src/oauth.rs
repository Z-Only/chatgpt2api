use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::error::{AppError, AppResult};

pub const DEFAULT_LOGIN_CALLBACK_PORT: u16 = 1455;
const PKCE_VERIFIER_BYTES: usize = 32;
const MIN_DEVICE_POLL_INTERVAL_SECS: u64 = 5;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
    pub method: String,
}

impl PkcePair {
    pub fn generate() -> AppResult<Self> {
        Self::from_verifier(random_code_verifier()?)
    }

    pub fn from_verifier(verifier: impl Into<String>) -> AppResult<Self> {
        let verifier = verifier.into();
        validate_code_verifier(&verifier)?;

        Ok(Self {
            challenge: pkce_challenge(&verifier),
            verifier,
            method: "S256".to_string(),
        })
    }
}

pub fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct JwtClaims {
    pub sub: Option<String>,
    pub email: Option<String>,
    pub exp: Option<i64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub fn parse_jwt_claims(token: &str) -> AppResult<JwtClaims> {
    let mut parts = token.split('.');
    let header = parts.next();
    let payload = parts.next();
    let signature = parts.next();
    if header.is_none() || payload.is_none() || signature.is_none() || parts.next().is_some() {
        return Err(AppError::Auth("JWT must have three segments".to_string()));
    }

    let payload = payload.unwrap();
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| AppError::Auth(format!("invalid JWT payload encoding: {error}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| AppError::Auth(format!("invalid JWT payload JSON: {error}")))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

pub struct BrowserCallbackServer {
    listener: TcpListener,
    expected_state: String,
}

impl BrowserCallbackServer {
    pub async fn bind(port: u16, expected_state: impl Into<String>) -> AppResult<Self> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        let listener = TcpListener::bind(addr).await?;
        Ok(Self::new(listener, expected_state))
    }

    pub async fn bind_default(expected_state: impl Into<String>) -> AppResult<Self> {
        Self::bind(DEFAULT_LOGIN_CALLBACK_PORT, expected_state).await
    }

    pub fn new(listener: TcpListener, expected_state: impl Into<String>) -> Self {
        Self {
            listener,
            expected_state: expected_state.into(),
        }
    }

    pub async fn wait_for_callback(self) -> AppResult<OAuthCallback> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let state = CallbackState {
            expected_state: self.expected_state,
            callback: Arc::new(Mutex::new(Some(callback_tx))),
            shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
        };
        let app = Router::new()
            .route("/callback", get(callback_handler))
            .with_state(state);

        axum::serve(self.listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await?;

        callback_rx
            .await
            .map_err(|_| AppError::Auth("login callback closed".to_string()))?
    }
}

#[derive(Clone)]
struct CallbackState {
    expected_state: String,
    callback: Arc<Mutex<Option<oneshot::Sender<AppResult<OAuthCallback>>>>>,
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn callback_handler(
    State(state): State<CallbackState>,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let result = match (query.code, query.state, query.error) {
        (_, _, Some(error)) => Err(AppError::Auth(format!("login failed: {error}"))),
        (Some(code), Some(actual_state), None) if actual_state == state.expected_state => {
            Ok(OAuthCallback {
                code,
                state: actual_state,
            })
        }
        (Some(_), Some(_), None) => Err(AppError::Auth("login state mismatch".to_string())),
        _ => Err(AppError::Auth("login callback missing code".to_string())),
    };

    let status = if result.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    if let Some(sender) = state
        .callback
        .lock()
        .expect("callback lock poisoned")
        .take()
    {
        let _ = sender.send(result);
    }
    if let Some(sender) = state
        .shutdown
        .lock()
        .expect("callback shutdown lock poisoned")
        .take()
    {
        let _ = sender.send(());
    }

    (status, Html("Login complete. You can close this window.")).into_response()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceCodeLogin {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub poll_interval_secs: u64,
}

impl DeviceCodeLogin {
    pub fn from_response(response: DeviceCodeResponse, now: DateTime<Utc>) -> AppResult<Self> {
        if response.device_code.trim().is_empty() {
            return Err(AppError::Auth("device code is empty".to_string()));
        }
        if response.user_code.trim().is_empty() {
            return Err(AppError::Auth("user code is empty".to_string()));
        }
        if response.verification_uri.trim().is_empty() {
            return Err(AppError::Auth("verification URI is empty".to_string()));
        }

        let expires_in = i64::try_from(response.expires_in)
            .map_err(|_| AppError::Auth("device code expiry is too large".to_string()))?;

        Ok(Self {
            device_code: response.device_code,
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            verification_uri_complete: response.verification_uri_complete,
            expires_at: now + Duration::seconds(expires_in),
            poll_interval_secs: response
                .interval
                .unwrap_or(MIN_DEVICE_POLL_INTERVAL_SECS)
                .max(MIN_DEVICE_POLL_INTERVAL_SECS),
        })
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

fn random_code_verifier() -> AppResult<String> {
    let mut bytes = [0; PKCE_VERIFIER_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Auth(format!("random generator failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn validate_code_verifier(verifier: &str) -> AppResult<()> {
    if !(43..=128).contains(&verifier.len()) {
        return Err(AppError::Auth(
            "PKCE verifier must be 43..=128 characters".to_string(),
        ));
    }

    if verifier
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
    {
        return Ok(());
    }

    Err(AppError::Auth(
        "PKCE verifier contains invalid characters".to_string(),
    ))
}
