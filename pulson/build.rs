// pulson/build.rs

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../pulson-ui/src");
    println!("cargo:rerun-if-changed=../pulson-ui/static/index.html");
    // Monitor all modular CSS files
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/base.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/auth.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/dashboard.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/settings.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/pulse_visualization.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/image_visualization.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/sensor_visualization.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/event_visualization.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/trigger_visualization.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/inline_map.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/mobile.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/pwa.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/logo.svg");
    println!("cargo:rerun-if-changed=../pulson-ui/static/logo.png");
    println!("cargo:rerun-if-changed=../pulson-ui/static/manifest.json");
    println!("cargo:rerun-if-changed=../pulson-ui/static/sw.js");
    println!("cargo:rerun-if-changed=../pulson-ui/static/pwa-manager.js");
    println!("cargo:rerun-if-changed=../pulson-ui/static/offline.html");
    println!("cargo:rerun-if-changed=../pulson-ui/static/pwa-test.html");
    println!("cargo:rerun-if-changed=../pulson-ui/static/test-map.html");
    println!("cargo:rerun-if-changed=../pulson-ui/static/test-wasm-map.html");

    // 1) Get the crate root as a PathBuf
    let pulson_manifest = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let pulson_root = PathBuf::from(&pulson_manifest);

    // 2) Compute the UI directory (sibling of pulson/)
    let parent_dir = pulson_root
        .parent()
        .expect("pulson has no parent directoryy");
    let ui_dir = parent_dir.join("pulson-ui");
    let dist_dir = ui_dir.join("ui").join("dist");
    let static_dir = ui_dir.join("static");
    let static_styles_dir = static_dir.join("styles");
    let dist_styles_dir = dist_dir.join("styles");
    
    // Static files
    let static_index = static_dir.join("index.html");
    let static_logo_svg = static_dir.join("logo.svg");
    let static_logo_png = static_dir.join("logo.png");
    let static_manifest = static_dir.join("manifest.json");
    let static_sw = static_dir.join("sw.js");
    let static_pwa_manager = static_dir.join("pwa-manager.js");
    let static_offline = static_dir.join("offline.html");
    let static_pwa_test = static_dir.join("pwa-test.html");
    
    // Dist files
    let dist_index = dist_dir.join("index.html");
    let dist_logo_svg = dist_dir.join("logo.svg");
    let dist_logo_png = dist_dir.join("logo.png");
    let dist_manifest = dist_dir.join("manifest.json");
    let dist_sw = dist_dir.join("sw.js");
    let dist_pwa_manager = dist_dir.join("pwa-manager.js");
    let dist_offline = dist_dir.join("offline.html");
    let dist_pwa_test = dist_dir.join("pwa-test.html");
    
    // CSS files
    let css_files = [
        "base.css",
        "auth.css", 
        "dashboard.css",
        "settings.css",
        "pulse_visualization.css",
        "image_visualization.css",
        "sensor_visualization.css",
        "event_visualization.css",
        "trigger_visualization.css",
        "inline_map.css",
        "mobile.css",
        "pwa.css"
    ];

    // 3) Build the UI via wasm-pack
    let status = Command::new("wasm-pack")
        .env("CARGO_TARGET_DIR", "../target/target-wasm")
        .args(&[
            "build",
            "../pulson-ui",
            "--release",
            "--target",
            "web",
            "--out-dir",
            "ui/dist",
        ])
        .current_dir(&ui_dir)
        .status()
        .expect("failed to run wasm-pack");
    if !status.success() {
        panic!("wasm-pack build failed");
    }

    fs::copy(static_index, dist_index).expect("failed to copy index.html");
    fs::copy(static_logo_svg, dist_logo_svg).expect("failed to copy logo.svg");
    fs::copy(static_logo_png, dist_logo_png).expect("failed to copy logo.png");
    
    // Copy PWA files
    fs::copy(static_manifest, dist_manifest).expect("failed to copy manifest.json");
    fs::copy(static_sw, dist_sw).expect("failed to copy sw.js");
    fs::copy(static_pwa_manager, dist_pwa_manager).expect("failed to copy pwa-manager.js");
    fs::copy(static_offline, dist_offline).expect("failed to copy offline.html");
    fs::copy(static_pwa_test, dist_pwa_test).expect("failed to copy pwa-test.html");
    
    // Create styles directory in dist
    fs::create_dir_all(&dist_styles_dir).expect("failed to create styles directory");
    
    // Copy all CSS files
    for css_file in &css_files {
        let src = static_styles_dir.join(css_file);
        let dst = dist_styles_dir.join(css_file);
        fs::copy(src, dst).expect(&format!("failed to copy {}", css_file));
    }
}
