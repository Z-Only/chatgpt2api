use std::sync::{Arc, RwLock};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState {
    config: Arc<RwLock<AppConfig>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
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
}
