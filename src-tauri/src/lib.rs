pub mod api;
pub mod app_state;
pub mod auth;
pub mod config;
pub mod error;
pub mod image;
pub mod models;
pub mod oauth;
pub mod rate_limits;
pub mod reasoning;
pub mod server;
pub mod session;
pub mod sse;
pub mod tools;
pub mod transform;
pub mod upstream;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg_attr(not(all(debug_assertions, feature = "pilot")), allow(unused_mut))]
    let mut builder = tauri::Builder::default();

    #[cfg(all(debug_assertions, feature = "pilot"))]
    {
        builder = builder.plugin(tauri_plugin_pilot::init());
    }

    builder
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
