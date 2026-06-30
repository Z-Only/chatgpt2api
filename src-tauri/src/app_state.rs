use std::sync::{Arc, Mutex, RwLock};

use crate::api::ApiState;
use crate::auth::{
    load_local_chatgpt_credentials, load_local_chatgpt_credentials_from_path,
    LocalChatGptCredentials,
};
use crate::config::AppConfig;
use crate::error::AppResult;
use crate::server::{self, ServerHandle};
use std::path::Path;

#[derive(Clone, Debug, serde::Serialize)]
pub struct ServerStatus {
    pub running: bool,
    pub host: String,
    pub port: u16,
    pub url: String,
}

#[derive(Clone)]
pub struct AppState {
    config: Arc<RwLock<AppConfig>>,
    server: Arc<Mutex<Option<ServerHandle>>>,
    credentials: Arc<RwLock<Option<LocalChatGptCredentials>>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            server: Arc::new(Mutex::new(None)),
            credentials: Arc::new(RwLock::new(None)),
        }
    }

    pub fn config(&self) -> AppConfig {
        self.config
            .read()
            .expect("app config lock poisoned")
            .clone()
    }

    pub fn set_config(&self, config: AppConfig) {
        *self.config.write().expect("app config lock poisoned") = config;
    }

    pub fn login_local_credentials(&self) -> AppResult<LocalChatGptCredentials> {
        let credentials = load_local_chatgpt_credentials()?;
        self.set_credentials(credentials.clone());
        Ok(credentials)
    }

    pub fn login_local_credentials_from_path(
        &self,
        path: &Path,
    ) -> AppResult<LocalChatGptCredentials> {
        let credentials = load_local_chatgpt_credentials_from_path(path)?;
        self.set_credentials(credentials.clone());
        Ok(credentials)
    }

    pub fn credentials(&self) -> Option<LocalChatGptCredentials> {
        self.credentials
            .read()
            .expect("credentials lock poisoned")
            .clone()
    }

    pub async fn start_server(&self) -> AppResult<ServerStatus> {
        if self.server.lock().expect("server lock poisoned").is_some() {
            return Ok(self.server_status());
        }

        let config = self.config();
        let api_state = if let Some(credentials) = self.credentials() {
            let upstream = credentials.upstream_client(&config.api.upstream_base_url)?;
            ApiState::with_upstream(config, upstream)
        } else {
            ApiState::new(config)
        };
        let handle = server::spawn_with_state(api_state).await?;
        let status = ServerStatus {
            running: true,
            host: handle.addr().ip().to_string(),
            port: handle.addr().port(),
            url: format!("http://{}", handle.addr()),
        };
        *self.server.lock().expect("server lock poisoned") = Some(handle);

        Ok(status)
    }

    pub fn stop_server(&self) -> AppResult<()> {
        if let Some(handle) = self.server.lock().expect("server lock poisoned").take() {
            handle.stop();
        }
        Ok(())
    }

    pub fn server_status(&self) -> ServerStatus {
        if let Some(handle) = self.server.lock().expect("server lock poisoned").as_ref() {
            return ServerStatus {
                running: true,
                host: handle.addr().ip().to_string(),
                port: handle.addr().port(),
                url: format!("http://{}", handle.addr()),
            };
        }

        let config = self.config();
        ServerStatus {
            running: false,
            host: config.server.host.clone(),
            port: config.server.port,
            url: format!("http://{}:{}", config.server.host, config.server.port),
        }
    }

    fn set_credentials(&self, credentials: LocalChatGptCredentials) {
        *self.credentials.write().expect("credentials lock poisoned") = Some(credentials);
    }
}
