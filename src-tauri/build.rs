fn main() {
    for icon in [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico",
    ] {
        println!("cargo:rerun-if-changed={icon}");
    }

    let pilot_capability_enabled =
        std::env::var_os("CARGO_FEATURE_PILOT").is_some() && cfg!(debug_assertions);
    let attributes = if pilot_capability_enabled {
        tauri_build::Attributes::new()
    } else {
        tauri_build::Attributes::new().capabilities_path_pattern("./capabilities/default.json")
    };

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
