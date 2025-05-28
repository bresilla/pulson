use directories::ProjectDirs;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::{fs, io};
use crate::logic::client::url_utils::build_api_url;

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
    base_url: Option<String>,
    host: String,
    port: u16,
    username: String,
    password: String,
    rootpass: Option<String>,
) -> anyhow::Result<()> {
    let url = build_api_url(base_url.as_deref(), &host, port, "/api/account/register");
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
    base_url: Option<String>,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> anyhow::Result<()> {
    let url = build_api_url(base_url.as_deref(), &host, port, "/api/account/login");
    let payload = AccountPayload {
        username: &username,
        password: &password,
        rootpass: None,
    };
    let resp = Client::new().post(&url).json(&payload).send().await?;

    if resp.status().is_success() {
        let json: Value = resp.json().await?;
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

pub async fn logout(
    base_url: Option<String>,
    host: String,
    port: u16,
) -> anyhow::Result<()> {
    let token = match read_token() {
        Ok(t) => t,
        Err(_) => {
            println!("⚠ Not logged in, no local token to clear.");
            return Ok(());
        }
    };

    let url = build_api_url(base_url.as_deref(), &host, port, "/api/account/logout");
    let client = Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .send()
        .await?;

    if resp.status().is_success() {
        match fs::remove_file(token_file()?) {
            Ok(_) => println!("✓ Logged out successfully and local token cleared."),
            Err(e) => {
                eprintln!("✓ Logged out from server, but failed to remove local token: {}. Please remove it manually.", e);
            }
        }
    } else {
        let status = resp.status();
        let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("✗ Logout failed on server (status: {}): {}", status, error_text);
        eprintln!("ℹ Your local token was not cleared. You might still be logged in on the server or the token might be invalid.");
    }

    Ok(())
}

pub async fn delete(
    base_url: Option<String>,
    host: String,
    port: u16,
    target: String,
) -> anyhow::Result<()> {
    let token = match read_token() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("✗ Not logged in");
            return Ok(());
        }
    };
    let url = build_api_url(base_url.as_deref(), &host, port, &format!("/api/account/{}", target));
    let resp = Client::new().delete(&url).bearer_auth(token).send().await?;
    if resp.status().is_success() {
        println!("✓ Deleted user `{}`", target);
    } else {
        eprintln!("✗ Delete failed: {}", resp.status());
    }
    Ok(())
}

/// List all users (must be root)
pub async fn list_users(
    base_url: Option<String>,
    host: String,
    port: u16,
) -> anyhow::Result<()> {
    let token = match read_token() {
        Ok(t) => t,
        Err(_) => {
            eprintln!("✗ Not logged in");
            return Ok(());
        }
    };

    let url = build_api_url(base_url.as_deref(), &host, port, "/api/account/users");
    let resp = Client::new().get(&url).bearer_auth(token).send().await?;

    if !resp.status().is_success() {
        eprintln!("✗ Failed: HTTP {}", resp.status());
        return Ok(());
    }

    let text = resp.text().await?;
    let users_arr: Vec<Value> = if text.trim().is_empty() {
        Vec::new()
    } else {
        let json_val: Value = serde_json::from_str(&text)?;
        if let Some(arr) = json_val.as_array() {
            arr.clone()
        } else if let Some(arr) = json_val.get("users").and_then(|v| v.as_array()) {
            arr.clone()
        } else {
            anyhow::bail!("Unexpected response format: {}", json_val);
        }
    };

    if users_arr.is_empty() {
        println!("No users found.");
        return Ok(());
    }

    println!("{:<20} ROLE", "USERNAME");
    for u in users_arr {
        let name = u["username"].as_str().unwrap_or("<invalid>");
        let role = u["role"].as_str().unwrap_or("<invalid>");
        println!("{:<20} {}", name, role);
    }
    Ok(())
}