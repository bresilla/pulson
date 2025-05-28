use crate::logic::config::StatusConfig;
use crate::logic::client::account::read_token;
use crate::logic::client::url_utils::build_api_url;
use crate::cli::HostConfig;
use colored::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Get host configuration from environment variable or default
fn get_host_config() -> HostConfig {
    let host_input = std::env::var("PULSON_HOST").unwrap_or_else(|_| "127.0.0.1:3030".to_string());
    HostConfig::from_str(&host_input).unwrap_or_else(|e| {
        eprintln!("Error parsing PULSON_HOST '{}': {}", host_input, e);
        std::process::exit(1);
    })
}

#[derive(Deserialize)]
struct ConfigResponse {
    online_threshold_seconds: u64,
    warning_threshold_seconds: u64,
    stale_threshold_seconds: u64,
}

#[derive(Serialize)]
struct ConfigUpdateRequest {
    online_threshold_seconds: u64,
    warning_threshold_seconds: u64,
    stale_threshold_seconds: u64,
}

/// Show current user configuration from server
pub async fn show() -> anyhow::Result<()> {
    // Fetch user configuration from server
    match fetch_user_config().await {
        Ok(config) => {
            println!("{}", "Current User Configuration:".bright_blue().bold());
            display_config(&config);
        }
        Err(e) => {
            eprintln!("{} {}", "Error fetching user configuration:".red().bold(), e);
            println!();
            println!("{}", "Showing default configuration:".bright_blue().bold());
            let default_config = StatusConfig::default();
            display_config(&default_config);
            println!();
            println!("{}", "Note: Using default configuration. Set your personal configuration with 'pulson config set'.".yellow());
        }
    }

    Ok(())
}

fn display_config(config: &StatusConfig) {
    println!();
    println!("{}", "Current Thresholds:".bright_green().bold());
    println!("  {} {} seconds", "Online threshold:".cyan(), config.online_threshold_seconds.to_string().bright_white());
    println!("  {} {} seconds", "Warning threshold:".yellow(), config.warning_threshold_seconds.to_string().bright_white());
    println!("  {} {} seconds", "Stale threshold:".bright_red(), config.stale_threshold_seconds.to_string().bright_white());
    
    println!();
    println!("{}", "Status Definitions:".bright_green().bold());
    println!("  {} Device/topic has sent pings within the online threshold", "●".green());
    println!("  {} Device/topic has sent pings within the warning threshold", "●".yellow());
    println!("  {} Topics have sent pings within the stale threshold", "●".bright_red());
    println!("  {} Device/topic has not sent pings beyond the warning/stale threshold", "●".red());
}

/// Set user configuration on server
pub async fn set(
    online_threshold: Option<u64>,
    warning_threshold: Option<u64>,
    stale_threshold: Option<u64>,
) -> anyhow::Result<()> {
    if online_threshold.is_none() && warning_threshold.is_none() && stale_threshold.is_none() {
        eprintln!("{}", "Error: At least one threshold must be specified".red().bold());
        eprintln!("Use one or more of: --online-threshold, --warning-threshold, --stale-threshold");
        return Ok(());
    }

    // Get current user configuration from server
    let mut current_config = match fetch_user_config().await {
        Ok(config) => config,
        Err(_) => {
            println!("{}", "No existing user configuration found, using defaults as base".yellow());
            StatusConfig::default()
        }
    };

    // Update thresholds if provided
    if let Some(threshold) = online_threshold {
        current_config.online_threshold_seconds = threshold;
        println!("{} {} seconds", "Set online threshold to:".green(), threshold.to_string().bright_white());
    }
    
    if let Some(threshold) = warning_threshold {
        current_config.warning_threshold_seconds = threshold;
        println!("{} {} seconds", "Set warning threshold to:".yellow(), threshold.to_string().bright_white());
    }
    
    if let Some(threshold) = stale_threshold {
        current_config.stale_threshold_seconds = threshold;
        println!("{} {} seconds", "Set stale threshold to:".bright_red(), threshold.to_string().bright_white());
    }

    // Validate thresholds
    if current_config.online_threshold_seconds >= current_config.warning_threshold_seconds {
        eprintln!("{}", "Error: Online threshold must be less than warning threshold".red().bold());
        return Ok(());
    }
    
    if current_config.warning_threshold_seconds >= current_config.stale_threshold_seconds {
        eprintln!("{}", "Error: Warning threshold must be less than stale threshold".red().bold());
        return Ok(());
    }

    // Send configuration to server
    match update_user_config(&current_config).await {
        Ok(_) => {
            println!();
            println!("{}", "User configuration updated successfully".bright_green().bold());
            
            // Show updated configuration
            println!();
            show().await?;
        }
        Err(e) => {
            eprintln!("{} {}", "Failed to update user configuration:".red().bold(), e);
        }
    }

    Ok(())
}

/// Fetch user configuration from server
async fn fetch_user_config() -> anyhow::Result<StatusConfig> {
    let token = read_token()
        .map_err(|_| anyhow::anyhow!("Not logged in. Please run 'pulson account login' first."))?;

    let host_config = get_host_config();
    let url = build_api_url(host_config.base_url().as_deref(), &host_config.host, host_config.port, "/api/user/config");
    
    let client = Client::new();
    let response = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Server returned status: {}", response.status()));
    }

    let config_response: ConfigResponse = response.json().await?;
    
    Ok(StatusConfig {
        online_threshold_seconds: config_response.online_threshold_seconds,
        warning_threshold_seconds: config_response.warning_threshold_seconds,
        stale_threshold_seconds: config_response.stale_threshold_seconds,
    })
}

/// Update user configuration on server
async fn update_user_config(config: &StatusConfig) -> anyhow::Result<()> {
    let token = read_token()
        .map_err(|_| anyhow::anyhow!("Not logged in. Please run 'pulson account login' first."))?;

    let host_config = get_host_config();
    let url = build_api_url(host_config.base_url().as_deref(), &host_config.host, host_config.port, "/api/user/config");
    
    let request = ConfigUpdateRequest {
        online_threshold_seconds: config.online_threshold_seconds,
        warning_threshold_seconds: config.warning_threshold_seconds,
        stale_threshold_seconds: config.stale_threshold_seconds,
    };
    
    let client = Client::new();
    let response = client
        .post(&url)
        .bearer_auth(&token)
        .json(&request)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("Server returned status {}: {}", status, error_text));
    }

    Ok(())
}
