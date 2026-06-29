use chatgpt2api::config::AppConfig;
use chatgpt2api::models::{ModelRegistry, DEFAULT_MODEL};
use chatgpt2api::reasoning::parse_reasoning_suffix;

#[test]
fn models_normalize_gpt55_alias() {
    let parsed = parse_reasoning_suffix("gpt5.5").unwrap();

    assert_eq!(parsed.model, "gpt-5.5");
    assert_eq!(parsed.reasoning_effort, None);
}

#[test]
fn reasoning_suffix_extracts_effort() {
    let parsed = parse_reasoning_suffix("gpt-5.5-high").unwrap();

    assert_eq!(parsed.model, "gpt-5.5");
    assert_eq!(parsed.reasoning_effort.as_deref(), Some("high"));
}

#[test]
fn models_default_to_gpt55() {
    assert_eq!(DEFAULT_MODEL, "gpt-5.5");
    assert_eq!(ModelRegistry::default().default_model(), "gpt-5.5");
}

#[test]
fn models_config_default_is_used_when_request_omits_model() {
    let mut config = AppConfig::default();
    config.api.default_model = "gpt5.5".to_string();
    config.reasoning.effort = "low".to_string();

    let resolved = ModelRegistry::default().resolve(None, &config).unwrap();

    assert_eq!(resolved.model, "gpt-5.5");
    assert_eq!(resolved.reasoning_effort, "low");
}

#[test]
fn models_request_model_overrides_config_default() {
    let mut config = AppConfig::default();
    config.api.default_model = "other-model".to_string();
    config.reasoning.effort = "medium".to_string();

    let resolved = ModelRegistry::default()
        .resolve(Some("gpt-5.5-high"), &config)
        .unwrap();

    assert_eq!(resolved.model, "gpt-5.5");
    assert_eq!(resolved.reasoning_effort, "high");
}

#[test]
fn models_public_list_can_include_reasoning_variants() {
    let mut config = AppConfig::default();
    config.api.expose_reasoning_models = true;

    let models = ModelRegistry::from_config(&config).public_models();

    assert!(models.contains(&"gpt-5.5".to_string()));
    assert!(models.contains(&"gpt-5.5-high".to_string()));
}
