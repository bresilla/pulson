use std::process::Command;

fn main() {
    // build the WASM-UI
    let status = Command::new("wasm-pack")
        .args(&[
            "build",
            "../pulson-ui",
            "--release",
            "--target",
            "web",
            "--out-dir",
            "pulson-ui/ui/dist",
        ])
        .status()
        .expect("failed to run wasm-pack");
    if !status.success() {
        panic!("wasm-pack failed");
    }

    // Tell Cargo to re-run build.rs if UI sources change:
    println!("cargo:rerun-if-changed=../pulson-ui/src");
    println!("cargo:rerun-if-changed=../pulson-ui/static");
}
