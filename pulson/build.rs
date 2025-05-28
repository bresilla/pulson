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
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/inline_map.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/styles/mobile.css");
    println!("cargo:rerun-if-changed=../pulson-ui/static/logo.svg");
    println!("cargo:rerun-if-changed=../pulson-ui/static/logo.png");
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
    
    // Dist files
    let dist_index = dist_dir.join("index.html");
    let dist_logo_svg = dist_dir.join("logo.svg");
    let dist_logo_png = dist_dir.join("logo.png");
    
    // CSS files
    let css_files = [
        "base.css",
        "auth.css", 
        "dashboard.css",
        "settings.css",
        "pulse_visualization.css",
        "inline_map.css",
        "mobile.css"
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
    
    // Create styles directory in dist
    fs::create_dir_all(&dist_styles_dir).expect("failed to create styles directory");
    
    // Copy all CSS files
    for css_file in &css_files {
        let src = static_styles_dir.join(css_file);
        let dst = dist_styles_dir.join(css_file);
        fs::copy(src, dst).expect(&format!("failed to copy {}", css_file));
    }
}
