use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    app_state::{AppState, ServerStatus},
    config::AppConfig,
};

#[derive(Clone, Debug, Serialize)]
pub struct AccountInfo {
    pub logged_in: bool,
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UsageLimits {
    pub limits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ImageCommandRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub background: Option<String>,
    pub output_format: Option<String>,
    pub output_compression: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImageCommandResponse {
    pub b64_json: String,
}

#[tauri::command]
pub async fn login_browser(state: State<'_, AppState>) -> Result<AccountInfo, String> {
    state
        .login_local_credentials()
        .map(credentials_account_info)
        .map_err(to_message)
}

#[tauri::command]
pub async fn logout() -> Result<AccountInfo, String> {
    Ok(logged_out_account())
}

#[tauri::command]
pub async fn account_info(state: State<'_, AppState>) -> Result<AccountInfo, String> {
    Ok(state
        .credentials()
        .map(credentials_account_info)
        .unwrap_or_else(logged_out_account))
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let path = AppConfig::default_config_path().map_err(to_message)?;
    let config = AppConfig::load_or_create_at(&path).map_err(to_message)?;
    state.set_config(config.clone());
    Ok(config)
}

#[tauri::command]
pub fn save_config(config: AppConfig, state: State<'_, AppState>) -> Result<AppConfig, String> {
    config.validate().map_err(to_message)?;
    let path = AppConfig::default_config_path().map_err(to_message)?;
    config.save_to_path(&path).map_err(to_message)?;
    state.set_config(config.clone());
    Ok(config)
}

#[tauri::command]
pub async fn start_server(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    state.start_server().await.map_err(to_message)
}

#[tauri::command]
pub fn stop_server(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    state.stop_server().map_err(to_message)?;
    Ok(state.server_status())
}

#[tauri::command]
pub fn server_status(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    Ok(state.server_status())
}

#[tauri::command]
pub async fn usage_limits() -> Result<UsageLimits, String> {
    Ok(UsageLimits { limits: Vec::new() })
}

#[tauri::command]
pub async fn generate_image(request: ImageCommandRequest) -> Result<ImageCommandResponse, String> {
    let _ = request;
    Err("Image upstream is not configured yet".to_string())
}

#[tauri::command]
pub async fn edit_image(request: ImageCommandRequest) -> Result<ImageCommandResponse, String> {
    let _ = request;
    Err("Image upstream is not configured yet".to_string())
}

#[tauri::command]
pub async fn stream_logs() -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub fn config_path() -> Result<String, String> {
    AppConfig::default_config_path()
        .map(|path| path.display().to_string())
        .map_err(to_message)
}

fn logged_out_account() -> AccountInfo {
    AccountInfo {
        logged_in: false,
        email: None,
        account_id: None,
        expires_at: None,
    }
}

fn credentials_account_info(credentials: crate::auth::LocalChatGptCredentials) -> AccountInfo {
    AccountInfo {
        logged_in: true,
        email: credentials.email,
        account_id: credentials.account_id,
        expires_at: Some(credentials.expires_at.to_rfc3339()),
    }
}

fn to_message(error: impl ToString) -> String {
    error.to_string()
}
