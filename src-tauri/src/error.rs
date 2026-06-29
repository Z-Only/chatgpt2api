pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML decode error: {0}")]
    TomlDecode(#[from] toml::de::Error),

    #[error("TOML encode error: {0}")]
    TomlEncode(#[from] toml::ser::Error),

    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("upstream error: {0}")]
    Upstream(String),
}
