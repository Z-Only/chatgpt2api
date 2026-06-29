// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    if std::env::args_os().len() > 1 {
        if let Err(error) = chatgpt2api::cli::run().await {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    chatgpt2api::run();
}
