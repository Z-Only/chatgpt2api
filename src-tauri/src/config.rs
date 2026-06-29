use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub api: ApiConfig,
    pub reasoning: ReasoningConfig,
    pub text: TextConfig,
    pub image: ImageConfig,
    pub features: FeatureConfig,
    pub ui: UiConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub login_callback_port: u16,
    pub allow_external_bind: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ApiConfig {
    pub default_model: String,
    pub expose_reasoning_models: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ReasoningConfig {
    pub effort: String,
    pub summary: Option<String>,
    pub compat: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TextConfig {
    pub verbosity: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ImageConfig {
    pub default_model: String,
    pub size: String,
    pub quality: String,
    pub background: String,
    pub output_format: String,
    pub output_compression: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct FeatureConfig {
    pub fast_mode: bool,
    pub enable_web_search: bool,
    pub enable_image_api: bool,
    pub enable_responses_websocket: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct UiConfig {
    pub locale: String,
    pub theme: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeOverrides {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub sets: Vec<(String, String)>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 14550,
            login_callback_port: 1455,
            allow_external_bind: false,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            default_model: "gpt-5.5".to_string(),
            expose_reasoning_models: true,
        }
    }
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            effort: "medium".to_string(),
            summary: Some("auto".to_string()),
            compat: "hidden".to_string(),
        }
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            verbosity: "medium".to_string(),
        }
    }
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            default_model: "chatgpt-image-latest".to_string(),
            size: "auto".to_string(),
            quality: "auto".to_string(),
            background: "auto".to_string(),
            output_format: "png".to_string(),
            output_compression: None,
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            fast_mode: false,
            enable_web_search: true,
            enable_image_api: true,
            enable_responses_websocket: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            locale: "system".to_string(),
            theme: "system".to_string(),
        }
    }
}

impl AppConfig {
    pub fn config_path_for_home(home: &Path) -> PathBuf {
        home.join(".chatgpt2api").join("config.toml")
    }

    pub fn default_config_path() -> AppResult<PathBuf> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| AppError::InvalidConfig("home directory not found".to_string()))?;
        Ok(Self::config_path_for_home(Path::new(&home)))
    }

    pub fn from_toml_str(input: &str) -> AppResult<Self> {
        let config: Self = toml::from_str(input)?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml_string(&self) -> AppResult<String> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn load_from_path(path: &Path) -> AppResult<Self> {
        Self::from_toml_str(&fs::read_to_string(path)?)
    }

    pub fn load_or_create_at(path: &Path) -> AppResult<Self> {
        if path.exists() {
            return Self::load_from_path(path);
        }

        let config = Self::default();
        config.save_to_path(path)?;
        Ok(config)
    }

    pub fn save_to_path(&self, path: &Path) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(path, self.to_toml_string()?)?;
        set_user_only_permissions(path)?;
        Ok(())
    }

    pub fn load_for_runtime(path: &Path, overrides: RuntimeOverrides) -> AppResult<Self> {
        Self::load_for_runtime_with_env(path, overrides, |key| std::env::var(key).ok())
    }

    pub fn load_for_runtime_with_env<F>(
        path: &Path,
        overrides: RuntimeOverrides,
        env: F,
    ) -> AppResult<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut config = if path.exists() {
            Self::load_from_path(path)?
        } else {
            Self::default()
        };

        if let Some(port) = env("CHATGPT2API_PORT") {
            config.server.port = parse_port(&port, "CHATGPT2API_PORT")?;
        }

        if let Some(host) = overrides.host {
            config.server.host = host;
        }
        if let Some(port) = overrides.port {
            config.server.port = port;
        }
        for (key, value) in overrides.sets {
            config.apply_set(&key, &value)?;
        }

        config.validate()?;
        Ok(config)
    }

    pub fn apply_set(&mut self, key: &str, value: &str) -> AppResult<()> {
        match key {
            "server.host" => self.server.host = value.to_string(),
            "server.port" | "port" => self.server.port = parse_port(value, key)?,
            "server.login_callback_port" => {
                self.server.login_callback_port = parse_port(value, key)?;
            }
            "server.allow_external_bind" => {
                self.server.allow_external_bind = parse_bool(value, key)?
            }
            "api.default_model" => self.api.default_model = value.to_string(),
            "api.expose_reasoning_models" => {
                self.api.expose_reasoning_models = parse_bool(value, key)?;
            }
            "reasoning.effort" => self.reasoning.effort = value.to_string(),
            "reasoning.summary" => self.reasoning.summary = Some(value.to_string()),
            "reasoning.compat" => self.reasoning.compat = value.to_string(),
            "text.verbosity" => self.text.verbosity = value.to_string(),
            "image.default_model" => self.image.default_model = value.to_string(),
            "image.size" => self.image.size = value.to_string(),
            "image.quality" => self.image.quality = value.to_string(),
            "image.background" => self.image.background = value.to_string(),
            "image.output_format" => self.image.output_format = value.to_string(),
            "image.output_compression" => {
                self.image.output_compression = if value.is_empty() {
                    None
                } else {
                    Some(value.parse().map_err(|_| invalid(key, "must be 0..=100"))?)
                };
            }
            "features.fast_mode" => self.features.fast_mode = parse_bool(value, key)?,
            "features.enable_web_search" => {
                self.features.enable_web_search = parse_bool(value, key)?;
            }
            "features.enable_image_api" => {
                self.features.enable_image_api = parse_bool(value, key)?;
            }
            "features.enable_responses_websocket" => {
                self.features.enable_responses_websocket = parse_bool(value, key)?;
            }
            "ui.locale" => self.ui.locale = value.to_string(),
            "ui.theme" => self.ui.theme = value.to_string(),
            _ => return Err(invalid(key, "unsupported config key")),
        }

        self.validate()
    }

    pub fn validate(&self) -> AppResult<()> {
        let host: IpAddr = self
            .server
            .host
            .parse()
            .map_err(|_| invalid("server.host", "must be an IP address"))?;
        if host.is_unspecified() && !self.server.allow_external_bind {
            return Err(invalid(
                "server.host",
                "external binds require allow_external_bind = true",
            ));
        }

        validate_port(self.server.port, "server.port")?;
        validate_port(
            self.server.login_callback_port,
            "server.login_callback_port",
        )?;
        validate_one(
            "reasoning.effort",
            &self.reasoning.effort,
            &["none", "minimal", "low", "medium", "high", "xhigh"],
        )?;
        if let Some(summary) = &self.reasoning.summary {
            validate_one(
                "reasoning.summary",
                summary,
                &["auto", "concise", "detailed"],
            )?;
        }
        validate_one(
            "reasoning.compat",
            &self.reasoning.compat,
            &["hidden", "think_tags", "summary"],
        )?;
        validate_one(
            "text.verbosity",
            &self.text.verbosity,
            &["low", "medium", "high"],
        )?;
        validate_one(
            "image.size",
            &self.image.size,
            &["auto", "1024x1024", "1024x1536", "1536x1024"],
        )?;
        validate_one(
            "image.quality",
            &self.image.quality,
            &["auto", "low", "medium", "high"],
        )?;
        validate_one(
            "image.background",
            &self.image.background,
            &["auto", "transparent", "opaque"],
        )?;
        validate_one(
            "image.output_format",
            &self.image.output_format,
            &["png", "jpeg", "webp"],
        )?;
        validate_image_compression(&self.image)?;
        validate_one("ui.locale", &self.ui.locale, &["system", "en", "zh"])?;
        validate_one("ui.theme", &self.ui.theme, &["system", "light", "dark"])?;
        Ok(())
    }
}

fn validate_image_compression(image: &ImageConfig) -> AppResult<()> {
    match (image.output_format.as_str(), image.output_compression) {
        ("png", Some(_)) => Err(invalid(
            "image.output_compression",
            "must be omitted for png output",
        )),
        ("jpeg" | "webp", Some(value)) if value > 100 => {
            Err(invalid("image.output_compression", "must be 0..=100"))
        }
        _ => Ok(()),
    }
}

fn validate_one(field: &str, value: &str, allowed: &[&str]) -> AppResult<()> {
    if allowed.contains(&value) {
        return Ok(());
    }

    Err(invalid(
        field,
        &format!("must be one of {}", allowed.join(", ")),
    ))
}

fn validate_port(port: u16, field: &str) -> AppResult<()> {
    if port == 0 {
        return Err(invalid(field, "must be 1..=65535"));
    }
    Ok(())
}

fn parse_bool(value: &str, field: &str) -> AppResult<bool> {
    value
        .parse()
        .map_err(|_| invalid(field, "must be true or false"))
}

fn parse_port(value: &str, field: &str) -> AppResult<u16> {
    let port = value
        .parse()
        .map_err(|_| invalid(field, "must be 1..=65535"))?;
    validate_port(port, field)?;
    Ok(port)
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::InvalidConfig(format!("{field} {message}"))
}

#[cfg(unix)]
fn set_user_only_permissions(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_user_only_permissions(_path: &Path) -> AppResult<()> {
    Ok(())
}
