use std::path::PathBuf;

use chatgpt2api::{cli::run_with_args_at, config::AppConfig};

fn temp_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("chatgpt2api-cli-{name}-{nanos}.toml"))
}

#[tokio::test]
async fn cli_models_lists_default_model() {
    let path = temp_path("models");
    let output = run_with_args_at(["chatgpt2api", "models"], &path)
        .await
        .unwrap();

    assert!(output.contains("gpt-5.5"));
}

#[tokio::test]
async fn cli_config_path_prints_config_path() {
    let path = temp_path("path");
    let output = run_with_args_at(["chatgpt2api", "config", "path"], &path)
        .await
        .unwrap();

    assert_eq!(output, path.display().to_string());
}

#[tokio::test]
async fn cli_config_set_persists_values() {
    let path = temp_path("set");

    run_with_args_at(["chatgpt2api", "config", "set", "port", "18080"], &path)
        .await
        .unwrap();
    run_with_args_at(
        ["chatgpt2api", "config", "set", "reasoning.effort", "high"],
        &path,
    )
    .await
    .unwrap();

    let config = AppConfig::load_from_path(&path).unwrap();
    assert_eq!(config.server.port, 18080);
    assert_eq!(config.reasoning.effort, "high");
}
