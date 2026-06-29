use std::sync::{Arc, Mutex, RwLock};

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::server::{self, ServerHandle};

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
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            server: Arc::new(Mutex::new(None)),
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

    pub async fn start_server(&self) -> AppResult<ServerStatus> {
        if self.server.lock().expect("server lock poisoned").is_some() {
            return Ok(self.server_status());
        }

        let handle = server::spawn(self.config()).await?;
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
}
