use std::fs;
use std::path::{Path, PathBuf};

use chatgpt2api::config::{AppConfig, RuntimeOverrides};

fn temp_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("chatgpt2api-{name}-{}-{nanos}", std::process::id()))
}

fn load_example() -> String {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::read_to_string(repo.join("config.example.toml")).unwrap()
}

#[test]
fn default_config_is_local_only() {
    let config = AppConfig::default();

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 14550);
    assert!(!config.server.allow_external_bind);
    assert_eq!(config.api.default_model, "gpt-5.5");
    config.validate().unwrap();
}

#[test]
fn config_invalid_values_fail_validation() {
    let mut config = AppConfig::default();
    config.server.port = 0;
    assert!(config.validate().is_err());

    let mut config = AppConfig::default();
    config.reasoning.effort = "extreme".to_string();
    assert!(config.validate().is_err());

    let mut config = AppConfig::default();
    config.text.verbosity = "verbose".to_string();
    assert!(config.validate().is_err());

    let mut config = AppConfig::default();
    config.image.output_compression = Some(20);
    assert!(config.validate().is_err());

    let mut config = AppConfig::default();
    config.ui.locale = "fr".to_string();
    assert!(config.validate().is_err());

    let mut config = AppConfig::default();
    config.ui.theme = "sepia".to_string();
    assert!(config.validate().is_err());
}

#[test]
fn config_external_bind_requires_explicit_flag() {
    let mut config = AppConfig::default();
    config.server.host = "0.0.0.0".to_string();

    assert!(config.validate().is_err());

    config.server.allow_external_bind = true;
    assert!(config.validate().is_ok());
}

#[test]
fn config_custom_port_persists_and_round_trips() {
    let path = temp_path("roundtrip").join("config.toml");
    let mut config = AppConfig::default();
    config.server.port = 18080;

    config.save_to_path(&path).unwrap();
    let loaded = AppConfig::load_from_path(&path).unwrap();

    assert_eq!(loaded.server.port, 18080);
    assert_eq!(loaded, config);
}

#[test]
fn config_example_parses() {
    let config = AppConfig::from_toml_str(&load_example()).unwrap();

    assert_eq!(config.server.port, 14550);
    assert_eq!(config.image.default_model, "chatgpt-image-latest");
}

#[test]
fn config_saves_to_chatgpt2api_home_path() {
    let home = temp_path("home");
    let path = AppConfig::config_path_for_home(&home);

    AppConfig::default().save_to_path(&path).unwrap();

    assert_eq!(path, home.join(".chatgpt2api").join("config.toml"));
    assert!(path.exists());
}

#[test]
fn config_file_never_includes_token_like_fields() {
    let toml = AppConfig::default()
        .to_toml_string()
        .unwrap()
        .to_lowercase();

    for field in [
        "token",
        "api_key",
        "access_token",
        "refresh_token",
        "id_token",
    ] {
        assert!(!toml.contains(field), "{field} leaked into config");
    }
}

#[test]
fn config_runtime_overrides_do_not_persist() {
    let path = temp_path("runtime").join("config.toml");
    AppConfig::default().save_to_path(&path).unwrap();

    let env_config =
        AppConfig::load_for_runtime_with_env(&path, RuntimeOverrides::default(), |key| {
            (key == "CHATGPT2API_PORT").then(|| "18080".to_string())
        })
        .unwrap();
    assert_eq!(env_config.server.port, 18080);

    let cli_config = AppConfig::load_for_runtime_with_env(
        &path,
        RuntimeOverrides {
            sets: vec![("reasoning.effort".to_string(), "high".to_string())],
            ..RuntimeOverrides::default()
        },
        |_| None,
    )
    .unwrap();
    assert_eq!(cli_config.reasoning.effort, "high");

    let saved = AppConfig::load_from_path(&path).unwrap();
    assert_eq!(saved.server.port, 14550);
    assert_eq!(saved.reasoning.effort, "medium");
}
