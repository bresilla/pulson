// pulson/build.rs

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../pulson-ui/src");
    println!("cargo:rerun-if-changed=../pulson-ui/static/index.html");
    println!("cargo:rerun-if-changed=../pulson-ui/static/style.css");
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
    let static_index = ui_dir.join("static").join("index.html");
    let static_css = ui_dir.join("static").join("style.css");
    let static_logo_svg = ui_dir.join("static").join("logo.svg");
    let static_logo_png = ui_dir.join("static").join("logo.png");
    let static_test_map = ui_dir.join("static").join("test-map.html");
    let static_test_wasm_map = ui_dir.join("static").join("test-wasm-map.html");
    let dist_index = dist_dir.join("index.html");
    let dist_css = dist_dir.join("style.css");
    let dist_logo_svg = dist_dir.join("logo.svg");
    let dist_logo_png = dist_dir.join("logo.png");
    let dist_test_map = dist_dir.join("test-map.html");
    let dist_test_wasm_map = dist_dir.join("test-wasm-map.html");

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
    fs::copy(static_css, dist_css).expect("failed to copy style.css");
    fs::copy(static_logo_svg, dist_logo_svg).expect("failed to copy logo.svg");
    fs::copy(static_logo_png, dist_logo_png).expect("failed to copy logo.png");
    fs::copy(static_test_map, dist_test_map).expect("failed to copy test-map.html");
    fs::copy(static_test_wasm_map, dist_test_wasm_map).expect("failed to copy test-wasm-map.html");
}
