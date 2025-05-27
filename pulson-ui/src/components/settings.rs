use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct ConfigData {
    pub online_threshold_seconds: u64,
    pub warning_threshold_seconds: u64,
    pub stale_threshold_seconds: u64,
}

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct UserData {
    pub username: String,
    pub is_root: bool,
}

#[derive(Serialize)]
struct ConfigUpdateRequest {
    online_threshold_seconds: u64,
    warning_threshold_seconds: u64,
    stale_threshold_seconds: u64,
}

#[function_component(Settings)]
pub fn settings() -> Html {
    let config_data = use_state(|| None::<ConfigData>);
    let user_data = use_state(|| None::<UserData>);
    let loading = use_state(|| true);
    let saving = use_state(|| false);
    let error = use_state(|| None::<String>);
    let success_message = use_state(|| None::<String>);
    let navigator = use_navigator().unwrap();

    // Form state
    let online_threshold = use_state(|| 30u64);
    let warning_threshold = use_state(|| 300u64);
    let stale_threshold = use_state(|| 3600u64);

    // Check if user is authenticated
    let token = LocalStorage::get::<String>("pulson_token").ok();
    if token.is_none() {
        navigator.push(&crate::Route::Login);
        return html! {};
    }

    // Fetch initial data
    {
        let config_data = config_data.clone();
        let user_data = user_data.clone();
        let loading = loading.clone();
        let error = error.clone();
        let online_threshold = online_threshold.clone();
        let warning_threshold = warning_threshold.clone();
        let stale_threshold = stale_threshold.clone();
        let token_clone = token.clone();

        use_effect_with_deps(
            move |_| {
                spawn_local(async move {
                    loading.set(true);
                    
                    // Fetch config and user data in parallel
                    let config_result = fetch_config_data().await;
                    let user_result = if let Some(auth_token) = token_clone {
                        fetch_user_data(&auth_token).await
                    } else {
                        Err("No token".to_string())
                    };
                    
                    match config_result {
                        Ok(config) => {
                            // Update form fields with current values
                            online_threshold.set(config.online_threshold_seconds);
                            warning_threshold.set(config.warning_threshold_seconds);
                            stale_threshold.set(config.stale_threshold_seconds);
                            config_data.set(Some(config));
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to load configuration: {}", e)));
                        }
                    }
                    
                    match user_result {
                        Ok(user) => {
                            user_data.set(Some(user));
                        }
                        Err(e) => {
                            gloo_console::error!("Failed to fetch user data:", e);
                        }
                    }
                    
                    loading.set(false);
                });
                || ()
            },
            (),
        );
    }

    // Navigation callbacks
    let go_to_dashboard = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&crate::Route::Dashboard);
        })
    };

    let logout = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            LocalStorage::delete("pulson_token");
            navigator.push(&crate::Route::Login);
        })
    };

    // Form input callbacks
    let on_online_threshold_change = {
        let online_threshold = online_threshold.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(value) = input.value().parse::<u64>() {
                online_threshold.set(value);
            }
        })
    };

    let on_warning_threshold_change = {
        let warning_threshold = warning_threshold.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(value) = input.value().parse::<u64>() {
                warning_threshold.set(value);
            }
        })
    };

    let on_stale_threshold_change = {
        let stale_threshold = stale_threshold.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(value) = input.value().parse::<u64>() {
                stale_threshold.set(value);
            }
        })
    };

    // Save configuration
    let on_save = {
        let config_data = config_data.clone();
        let saving = saving.clone();
        let error = error.clone();
        let success_message = success_message.clone();
        let online_threshold = online_threshold.clone();
        let warning_threshold = warning_threshold.clone();
        let stale_threshold = stale_threshold.clone();

        Callback::from(move |_| {
            let config_data = config_data.clone();
            let saving = saving.clone();
            let error = error.clone();
            let success_message = success_message.clone();
            let online_val = *online_threshold;
            let warning_val = *warning_threshold;
            let stale_val = *stale_threshold;

            spawn_local(async move {
                saving.set(true);
                error.set(None);
                success_message.set(None);

                // Validate thresholds
                if online_val >= warning_val {
                    error.set(Some("Online threshold must be less than warning threshold".to_string()));
                    saving.set(false);
                    return;
                }
                
                if warning_val >= stale_val {
                    error.set(Some("Warning threshold must be less than stale threshold".to_string()));
                    saving.set(false);
                    return;
                }

                // Update configuration
                match update_config_data(online_val, warning_val, stale_val).await {
                    Ok(_) => {
                        // Update local state
                        let new_config = ConfigData {
                            online_threshold_seconds: online_val,
                            warning_threshold_seconds: warning_val,
                            stale_threshold_seconds: stale_val,
                        };
                        config_data.set(Some(new_config));
                        success_message.set(Some("Configuration updated successfully!".to_string()));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to update configuration: {}", e)));
                    }
                }
                
                saving.set(false);
            });
        })
    };

    html! {
        <div class="settings-container">
            <aside class="sidebar">
                <div class="sidebar-header">
                    <img src="/static/logo.svg" alt="Pulson Logo" class="nav-logo" />
                    <h1>{"pulson"}</h1>
                </div>

                // User info moved to top of sidebar
                <div class="user-info-section">
                    <div class="user-info-display">
                        <div class="profile-image-placeholder"></div>
                        <div class="user-details">
                            <span class="username">
                                {
                                    if let Some(ud) = &*user_data {
                                        ud.username.clone()
                                    } else {
                                        "Loading...".to_string()
                                    }
                                }
                            </span>
                            <span class="user-role">
                                {
                                    if let Some(ud) = &*user_data {
                                        if ud.is_root {
                                            "(Root)".to_string()
                                        } else {
                                            "(User)".to_string()
                                        }
                                    } else {
                                        "".to_string()
                                    }
                                }
                            </span>
                        </div>
                    </div>
                    <button class="logout-button settings-logout" onclick={logout}>
                        {"Logout"}
                    </button>
                </div>

                <nav class="sidebar-nav">
                    <button class="nav-button back-to-dashboard" onclick={go_to_dashboard}>
                        {"← Back to Dashboard"}
                    </button>
                </nav>

                <div class="sidebar-footer">
                    // Footer can be used for additional controls if needed
                </div>
            </aside>

            <main class="settings-content">
                <div class="settings-header">
                    <h1>{"Settings"}</h1>
                    <p>{"Configure device status thresholds and other preferences"}</p>
                </div>

                if *loading {
                    <div class="loading-section">
                        <div class="loading-spinner"></div>
                        <p>{"Loading settings..."}</p>
                    </div>
                } else {
                    <div class="settings-sections">
                        // Threshold Configuration Section
                        <section class="settings-section">
                            <h2>{"Device Status Thresholds"}</h2>
                            <p class="section-description">
                                {"Configure when devices should be considered online, in warning state, or offline based on their last ping time."}
                            </p>

                            if let Some(err) = &*error {
                                <div class="error-message">
                                    <span class="error-icon">{"⚠"}</span>
                                    <span>{err}</span>
                                </div>
                            }

                            if let Some(msg) = &*success_message {
                                <div class="success-message">
                                    <span class="success-icon">{"✓"}</span>
                                    <span>{msg}</span>
                                </div>
                            }

                            <div class="threshold-form">
                                <div class="form-group">
                                    <label for="online-threshold" class="form-label">
                                        <span class="label-text">{"Online Threshold"}</span>
                                        <span class="label-unit">{"(seconds)"}</span>
                                    </label>
                                    <input
                                        id="online-threshold"
                                        type="number"
                                        min="1"
                                        max="3600"
                                        value={online_threshold.to_string()}
                                        oninput={on_online_threshold_change}
                                        class="form-input"
                                    />
                                    <p class="form-help">
                                        {"Devices are considered "}<span class="status-online">{"online"}</span>{" if they've pinged within this time."}
                                    </p>
                                </div>

                                <div class="form-group">
                                    <label for="warning-threshold" class="form-label">
                                        <span class="label-text">{"Warning Threshold"}</span>
                                        <span class="label-unit">{"(seconds)"}</span>
                                    </label>
                                    <input
                                        id="warning-threshold"
                                        type="number"
                                        min="1"
                                        max="7200"
                                        value={warning_threshold.to_string()}
                                        oninput={on_warning_threshold_change}
                                        class="form-input"
                                    />
                                    <p class="form-help">
                                        {"Devices show "}<span class="status-warning">{"warning"}</span>{" status if they haven't pinged within this time."}
                                    </p>
                                </div>

                                <div class="form-group">
                                    <label for="stale-threshold" class="form-label">
                                        <span class="label-text">{"Stale Threshold"}</span>
                                        <span class="label-unit">{"(seconds)"}</span>
                                    </label>
                                    <input
                                        id="stale-threshold"
                                        type="number"
                                        min="1"
                                        max="86400"
                                        value={stale_threshold.to_string()}
                                        oninput={on_stale_threshold_change}
                                        class="form-input"
                                    />
                                    <p class="form-help">
                                        {"Topics are considered "}<span class="status-stale">{"stale"}</span>{" if they haven't been active within this time."}
                                    </p>
                                </div>

                                <div class="threshold-preview">
                                    <h3>{"Current Configuration Preview"}</h3>
                                    <div class="preview-items">
                                        <div class="preview-item status-online">
                                            <span class="preview-dot"></span>
                                            <span>{"Online: 0 - "}{*online_threshold}{" seconds"}</span>
                                        </div>
                                        <div class="preview-item status-warning">
                                            <span class="preview-dot"></span>
                                            <span>{"Warning: "}{*online_threshold + 1}{" - "}{*warning_threshold}{" seconds"}</span>
                                        </div>
                                        <div class="preview-item status-offline">
                                            <span class="preview-dot"></span>
                                            <span>{"Offline: "}{*warning_threshold + 1}{" seconds and beyond"}</span>
                                        </div>
                                    </div>
                                </div>

                                <div class="form-actions">
                                    <button
                                        class={classes!("save-button", (*saving).then(|| "saving"))}
                                        onclick={on_save}
                                        disabled={*saving}
                                    >
                                        if *saving {
                                            <span class="button-spinner"></span>
                                            {"Saving..."}
                                        } else {
                                            {"Save Changes"}
                                        }
                                    </button>
                                </div>
                            </div>
                        </section>

                        // Additional Settings Sections (placeholder for future features)
                        <section class="settings-section">
                            <h2>{"Additional Settings"}</h2>
                            <p class="section-description">{"More configuration options will be available here in future releases."}</p>
                            
                            <div class="placeholder-settings">
                                <div class="placeholder-item">
                                    <span class="placeholder-label">{"Email Notifications"}</span>
                                    <span class="placeholder-status">{"Coming Soon"}</span>
                                </div>
                                <div class="placeholder-item">
                                    <span class="placeholder-label">{"Data Retention"}</span>
                                    <span class="placeholder-status">{"Coming Soon"}</span>
                                </div>
                                <div class="placeholder-item">
                                    <span class="placeholder-label">{"API Keys"}</span>
                                    <span class="placeholder-status">{"Coming Soon"}</span>
                                </div>
                            </div>
                        </section>
                    </div>
                }
            </main>
        </div>
    }
}

// API Functions
async fn fetch_user_data(token: &str) -> Result<UserData, String> {
    let request = Request::get("/api/userinfo")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<UserData>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}

async fn fetch_config_data() -> Result<ConfigData, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let request = Request::get("/api/user/config")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<ConfigData>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}

async fn update_config_data(online: u64, warning: u64, stale: u64) -> Result<(), String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let payload = ConfigUpdateRequest {
        online_threshold_seconds: online,
        warning_threshold_seconds: warning,
        stale_threshold_seconds: stale,
    };

    let request = Request::post("/api/user/config")
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&payload).map_err(|e| format!("Serialization error: {}", e))?)
        .map_err(|e| format!("Request creation error: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        Ok(())
    } else {
        let error_text = request
            .text()
            .await
            .unwrap_or_else(|_| format!("HTTP {}", request.status()));
        Err(error_text)
    }
}
