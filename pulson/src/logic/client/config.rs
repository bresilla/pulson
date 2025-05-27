use crate::logic::config::StatusConfig;
use std::path::Path;
use colored::*;

/// Show current configuration and thresholds
pub async fn show(config_path: Option<String>) -> anyhow::Result<()> {
    let config = if let Some(path) = config_path {
        println!("{} {}", "Loading configuration from:".bright_blue().bold(), path.bright_white());
        StatusConfig::from_file(&path)?
    } else {
        println!("{}", "Using default configuration:".bright_blue().bold());
        StatusConfig::default()
    };

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

    Ok(())
}

/// Set device status thresholds
pub async fn set(
    config_path: Option<String>,
    online_threshold: Option<u64>,
    warning_threshold: Option<u64>,
    stale_threshold: Option<u64>,
) -> anyhow::Result<()> {
    if online_threshold.is_none() && warning_threshold.is_none() && stale_threshold.is_none() {
        eprintln!("{}", "Error: At least one threshold must be specified".red().bold());
        eprintln!("Use one or more of: --online-threshold, --warning-threshold, --stale-threshold");
        return Ok(());
    }

    let config_file_path = config_path.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config/pulson/config.toml", home)
    });

    // Expand tilde if present
    let expanded_path = if config_file_path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        config_file_path.replace("~/", &format!("{}/", home))
    } else {
        config_file_path
    };

    // Load existing config or create default
    let mut config = if Path::new(&expanded_path).exists() {
        println!("{} {}", "Loading existing configuration from:".bright_blue().bold(), expanded_path.bright_white());
        StatusConfig::from_file(&expanded_path)?
    } else {
        println!("{} {}", "Creating new configuration file:".bright_green().bold(), expanded_path.bright_white());
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&expanded_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        StatusConfig::default()
    };

    // Update thresholds if provided
    if let Some(threshold) = online_threshold {
        config.online_threshold_seconds = threshold;
        println!("{} {} seconds", "Set online threshold to:".green(), threshold.to_string().bright_white());
    }
    
    if let Some(threshold) = warning_threshold {
        config.warning_threshold_seconds = threshold;
        println!("{} {} seconds", "Set warning threshold to:".yellow(), threshold.to_string().bright_white());
    }
    
    if let Some(threshold) = stale_threshold {
        config.stale_threshold_seconds = threshold;
        println!("{} {} seconds", "Set stale threshold to:".bright_red(), threshold.to_string().bright_white());
    }

    // Validate thresholds
    if config.online_threshold_seconds >= config.warning_threshold_seconds {
        eprintln!("{}", "Warning: Online threshold should be less than warning threshold".yellow().bold());
    }
    
    if config.warning_threshold_seconds >= config.stale_threshold_seconds {
        eprintln!("{}", "Warning: Warning threshold should be less than stale threshold".yellow().bold());
    }

    // Save the configuration
    config.save_to_file(&expanded_path)?;
    
    println!();
    println!("{} {}", "Configuration saved to:".bright_green().bold(), expanded_path.bright_white());
    
    // Try to notify the server to reload configuration
    if let Err(_) = notify_server_reload().await {
        println!("{}", "Note: Could not notify running server to reload configuration".yellow());
        println!("{}", "You may need to restart the server for changes to take effect".yellow());
    } else {
        println!("{}", "Server configuration reloaded successfully".bright_green());
    }
    
    // Show updated configuration
    println!();
    show(Some(expanded_path)).await?;

    Ok(())
}

/// Notify the server to reload its configuration
async fn notify_server_reload() -> anyhow::Result<()> {
    // Use default host/port or environment variables
    let host = std::env::var("PULSON_IP").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PULSON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3030); // Changed from 8080 to 3030 to match server default
    
    let url = format!("http://{}:{}/api/config/reload", host, port);
    
    let client = reqwest::Client::new();
    let response = client.post(&url).send().await?;
    
    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Server returned status: {}", response.status()))
    }
}
