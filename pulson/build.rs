use std::fs;
use std::process::Command;

/// This build script will:
/// 1) Run `wasm-pack build` on `pulson-ui`
/// 2) Copy the generated `index.html` into the embed folder
fn main() {
    // If UI sources change, rerun
    println!("cargo:rerun-if-changed=../pulson-ui/src");
    println!("cargo:rerun-if-changed=../pulson-ui/static/index.html");

    // 1) Run wasm-pack
    let status = Command::new("wasm-pack")
        .args(&[
            "build",
            "../pulson-ui", // path to the UI crate
            "--release",
            "--target",
            "web", // produce .js/.wasm for the web
            "--out-dir",
            "ui/dist", // relative to pulson-ui/
        ])
        .current_dir("../pulson-ui")
        .status()
        .expect("`wasm-pack` not found; install via `cargo install wasm-pack`");
    if !status.success() {
        panic!("wasm-pack build failed");
    }

    // 2) Overwrite the generated index.html with our static one
    // wasm-pack by default copies static assets from `static/`?
    // If not, explicitly copy:
    let from = "../pulson-ui/static/index.html";
    let to = "../pulson-ui/ui/dist/index.html";
    fs::copy(from, to).expect("Failed to copy index.html into ui/dist");
}
