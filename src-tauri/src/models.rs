use crate::config::AppConfig;
use crate::error::AppResult;
use crate::reasoning::{parse_reasoning_suffix, REASONING_MODEL_VARIANTS};

pub const DEFAULT_MODEL: &str = "gpt-5.5";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedModel {
    pub model: String,
    pub reasoning_effort: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRegistry {
    default_model: String,
    expose_reasoning_models: bool,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            default_model: DEFAULT_MODEL.to_string(),
            expose_reasoning_models: true,
        }
    }
}

impl ModelRegistry {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            default_model: config.api.default_model.clone(),
            expose_reasoning_models: config.api.expose_reasoning_models,
        }
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    pub fn public_models(&self) -> Vec<String> {
        let mut models = vec![self.default_model.clone()];
        if self.expose_reasoning_models {
            models.extend(
                REASONING_MODEL_VARIANTS
                    .iter()
                    .map(|effort| format!("{}-{effort}", self.default_model)),
            );
        }
        models
    }

    pub fn resolve(
        &self,
        request_model: Option<&str>,
        config: &AppConfig,
    ) -> AppResult<ResolvedModel> {
        let raw_model = request_model.unwrap_or(&config.api.default_model);
        let parsed = parse_reasoning_suffix(raw_model)?;

        Ok(ResolvedModel {
            model: parsed.model,
            reasoning_effort: parsed
                .reasoning_effort
                .unwrap_or_else(|| config.reasoning.effort.clone()),
        })
    }
}
