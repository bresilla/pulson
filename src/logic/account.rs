use directories::ProjectDirs;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::{fs, io};

#[derive(Serialize)]
struct AccountPayload<'a> {
    username: &'a str,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    rootpass: Option<&'a str>,
}

fn token_file() -> io::Result<std::path::PathBuf> {
    let pd = ProjectDirs::from("com", "example", "pulson")
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no config dir"))?;
    let dir = pd.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("token"))
}

pub fn read_token() -> io::Result<String> {
    let p = token_file()?;
    fs::read_to_string(p).map(|s| s.trim().to_string())
}

pub async fn register(
    host: String,
    port: u16,
    username: String,
    password: String,
    rootpass: Option<String>,
) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/account/register", host, port);
    let payload = AccountPayload {
        username: &username,
        password: &password,
        rootpass: rootpass.as_deref(),
    };
    let resp = Client::new().post(&url).json(&payload).send().await?;
    if resp.status().is_success() {
        println!("✓ Registered `{}`", username);
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
    let payload = AccountPayload {
        username: &username,
        password: &password,
        rootpass: None,
    };
    let resp = Client::new().post(&url).json(&payload).send().await?;

    if resp.status().is_success() {
        // specify Value so .json() knows what to parse
        let json: Value = resp.json::<Value>().await?;
        let tok = json["token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no token in response"))?
            .to_string();
        fs::write(token_file()?, &tok)?;
        println!("✓ Logged in");
    } else {
        eprintln!("✗ Login failed: {}", resp.text().await?);
    }
    Ok(())
}

pub fn logout() -> anyhow::Result<()> {
    let p = token_file()?;
    if p.exists() {
        fs::remove_file(p)?;
        println!("✓ Logged out");
    } else {
        println!("⚠ No token to remove");
    }
    Ok(())
}

pub async fn delete(host: String, port: u16, target: String) -> anyhow::Result<()> {
    let token = match read_token() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("✗ Not logged in");
            return Ok(());
        }
    };
    let url = format!("http://{}:{}/account/{}", host, port, target);
    let resp = Client::new().delete(&url).bearer_auth(token).send().await?;
    if resp.status().is_success() {
        println!("✓ Deleted user `{}`", target);
    } else {
        eprintln!("✗ Delete failed: {}", resp.status());
    }
    Ok(())
}
