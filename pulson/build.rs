// pulson/build.rs

use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../pulson-ui/src");
    println!("cargo:rerun-if-changed=../pulson-ui/static/index.html");

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
    let dist_index = dist_dir.join("index.html");

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

    // 4) Copy our custom index.html into the dist folder
    eprintln!(
        "📦 [build.rs] copying {:?} → {:?}",
        static_index, dist_index
    );
    fs::copy(static_index, dist_index).expect("failed to copy index.html");
}
