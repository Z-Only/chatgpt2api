use std::ffi::OsString;
use std::path::Path;

use clap::{error::ErrorKind, Parser, Subcommand};

use crate::{
    api::ApiState,
    config::{AppConfig, RuntimeOverrides},
    error::{AppError, AppResult},
    image::image_model_list,
    models::ModelRegistry,
    server,
    upstream::UpstreamClient,
};

#[derive(Debug, Parser)]
#[command(name = "chatgpt2api")]
#[command(about = "Local OpenAI-compatible API bridge for ChatGPT/Codex")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Login {
        #[arg(long)]
        headless: bool,
    },
    Logout,
    Serve {
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long = "set")]
        sets: Vec<String>,
    },
    Status,
    Models,
    Limits,
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Get,
    Set { key: String, value: String },
    Path,
}

pub async fn run() -> AppResult<()> {
    let path = AppConfig::default_config_path()?;
    let output = run_with_args_at(std::env::args_os(), &path).await?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

pub async fn run_with_args_at<I, T>(args: I, path: &Path) -> AppResult<String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return Ok(error.to_string());
        }
        Err(error) => return Err(AppError::InvalidRequest(error.to_string())),
    };

    match cli.command {
        Command::Login { headless } => {
            if headless {
                Ok("device login is not implemented yet".to_string())
            } else {
                Ok("browser login is not implemented yet".to_string())
            }
        }
        Command::Logout => Ok("logged out".to_string()),
        Command::Serve { host, port, sets } => serve(path, host, port, sets).await,
        Command::Status => {
            let config = AppConfig::load_or_create_at(path)?;
            Ok(format!(
                "stopped http://{}:{}",
                config.server.host, config.server.port
            ))
        }
        Command::Models => {
            let config = AppConfig::load_or_create_at(path)?;
            let mut models = ModelRegistry::from_config(&config).public_models();
            models.extend(image_model_list());
            Ok(models.join("\n"))
        }
        Command::Limits => Ok("[]".to_string()),
        Command::Config { command } => match command {
            ConfigCommand::Get => AppConfig::load_or_create_at(path)?.to_toml_string(),
            ConfigCommand::Set { key, value } => {
                let mut config = AppConfig::load_or_create_at(path)?;
                config.apply_set(&key, &value)?;
                config.save_to_path(path)?;
                Ok(format!("{key}={value}"))
            }
            ConfigCommand::Path => Ok(path.display().to_string()),
        },
    }
}

async fn serve(
    path: &Path,
    host: Option<String>,
    port: Option<u16>,
    sets: Vec<String>,
) -> AppResult<String> {
    let config = AppConfig::load_for_runtime(
        path,
        RuntimeOverrides {
            host,
            port,
            sets: parse_sets(sets)?,
        },
    )?;
    let handle = if let Some(state) = fake_upstream_state(config.clone())? {
        server::spawn_with_state(state).await?
    } else {
        server::spawn(config).await?
    };
    let url = format!("http://{}", handle.addr());
    tokio::signal::ctrl_c().await?;
    handle.stop();
    Ok(url)
}

fn parse_sets(sets: Vec<String>) -> AppResult<Vec<(String, String)>> {
    sets.into_iter()
        .map(|entry| {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| AppError::InvalidRequest("--set must be key=value".to_string()))?;
            Ok((key.to_string(), value.to_string()))
        })
        .collect()
}

fn fake_upstream_state(config: AppConfig) -> AppResult<Option<ApiState>> {
    let Ok(base_url) = std::env::var("CHATGPT2API_FAKE_UPSTREAM_BASE_URL") else {
        return Ok(None);
    };

    // ponytail: E2E-only fake upstream hook; replace with auth-built upstream clients when login is wired.
    let upstream = UpstreamClient::new(base_url, "e2e-token", "e2e-session", "e2e-install")?;
    let mut state = ApiState::with_upstream(config, upstream);
    if let Ok(ws_url) = std::env::var("CHATGPT2API_FAKE_UPSTREAM_WS_URL") {
        state = state.with_responses_ws_url(ws_url);
    }
    Ok(Some(state))
}
