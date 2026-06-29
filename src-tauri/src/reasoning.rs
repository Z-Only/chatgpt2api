use crate::error::{AppError, AppResult};

pub const REASONING_EFFORTS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];
pub const REASONING_MODEL_VARIANTS: &[&str] = &["minimal", "low", "medium", "high", "xhigh"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedModel {
    pub model: String,
    pub reasoning_effort: Option<String>,
}

pub fn parse_reasoning_suffix(model: &str) -> AppResult<ParsedModel> {
    let model = model.trim();
    if model.is_empty() {
        return Err(AppError::InvalidConfig(
            "model must not be empty".to_string(),
        ));
    }

    for effort in REASONING_EFFORTS {
        let suffix = format!("-{effort}");
        if let Some(base) = model.strip_suffix(&suffix) {
            if base.is_empty() {
                return Err(AppError::InvalidConfig(
                    "model must not be empty".to_string(),
                ));
            }
            return Ok(ParsedModel {
                model: normalize_model_name(base),
                reasoning_effort: Some((*effort).to_string()),
            });
        }
    }

    Ok(ParsedModel {
        model: normalize_model_name(model),
        reasoning_effort: None,
    })
}

pub fn normalize_model_name(model: &str) -> String {
    match model {
        "gpt5.5" => "gpt-5.5".to_string(),
        other => other.to_string(),
    }
}
