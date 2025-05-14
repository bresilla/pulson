use directories::ProjectDirs;
use reqwest::Client;
use serde::Serialize;
use std::{fs, io};

#[derive(Serialize)]
struct AccountPayload {
    username: String,
    password: String,
}

fn token_path() -> io::Result<std::path::PathBuf> {
    let pd = ProjectDirs::from("com", "example", "pulson")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "cannot find config dir"))?;
    let dir = pd.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("token"))
}

pub async fn register(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/account/register", host, port);
    let resp = Client::new()
        .post(&url)
        .json(&AccountPayload {
            username: username.clone(),
            password: password.clone(),
        })
        .send()
        .await?;
    if resp.status().is_success() {
        println!("✓ Registered user `{}`", username);
    } else {
        eprintln!("✗ Registration failed: {}", resp.text().await?);
    }
    Ok(())
}

pub async fn login(
    host: String,
    port: u16,
    username: String,
    password: String,
) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/account/login", host, port);
    let resp = Client::new()
        .post(&url)
        .json(&AccountPayload {
            username: username.clone(),
            password: password.clone(),
        })
        .send()
        .await?;
    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await?;
        let token = json
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("no token in response"))?;
        let path = token_path()?;
        fs::write(&path, token)?;
        println!("✓ Logged in, token saved to {:?}", path);
    } else {
        eprintln!("✗ Login failed: {}", resp.text().await?);
    }
    Ok(())
}

pub fn logout() -> anyhow::Result<()> {
    let path = token_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
        println!("✓ Logged out (removed token)");
    } else {
        println!("⚠️  No token to remove");
    }
    Ok(())
}
