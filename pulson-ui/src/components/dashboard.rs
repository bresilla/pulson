use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use js_sys::Date;
use serde::Deserialize;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, PartialEq, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_seen: String, // Keep as String since API returns mixed formats
    pub status: String, // Server-calculated status: "Online", "Warning", "Offline"
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub last_seen: String,
    pub status: String, // Server-calculated status: "Active", "Recent", "Stale", "Inactive"
}

#[derive(Clone, PartialEq, Deserialize, Debug)] // Added Debug for easier inspection
pub struct UserData {
    pub username: String,
    pub is_root: bool,
}

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct ConfigData {
    pub online_threshold_seconds: u64,
    pub warning_threshold_seconds: u64,
    pub stale_threshold_seconds: u64,
}

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let devices = use_state(Vec::<DeviceInfo>::new);
    let selected_device = use_state(|| None::<String>);
    let topics = use_state(Vec::<TopicInfo>::new);
    let selected_topic = use_state(|| None::<String>); // New state for selected topic
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let auto_refresh = use_state(|| true);
    let navigator = use_navigator().unwrap();
    let user_menu_visible = use_state(|| false);
    let user_data = use_state(|| None::<UserData>); // New state for user data
    let config_data = use_state(|| None::<ConfigData>); // New state for configuration data

    // Check if user is authenticated
    let token = LocalStorage::get::<String>("pulson_token").ok();
    if token.is_none() {
        navigator.push(&crate::Route::Login);
        return html! {};
    }

    // Fetch devices and config on component mount and when auto_refresh changes
    {
        let devices = devices.clone();
        let config_data = config_data.clone();
        let loading = loading.clone();
        let error = error.clone();
        let auto_refresh_val = *auto_refresh;

        use_effect_with_deps(
            move |_| {
                let devices_initial = devices.clone();
                let config_data_initial = config_data.clone();
                let loading_initial = loading.clone();
                let error_initial = error.clone();

                // Initial fetch
                spawn_local(async move {
                    loading_initial.set(true);
                    
                    // Fetch both devices and config in parallel
                    let devices_result = fetch_devices().await;
                    let config_result = fetch_config_data().await;
                    
                    match devices_result {
                        Ok(device_list) => {
                            devices_initial.set(device_list);
                            error_initial.set(None);
                        }
                        Err(e) => {
                            error_initial.set(Some(e));
                        }
                    }
                    
                    if let Ok(config) = config_result {
                        config_data_initial.set(Some(config));
                    }
                    
                    loading_initial.set(false);
                });

                // Set up auto-refresh interval if enabled
                let interval_handle = if auto_refresh_val {
                    let interval_devices = devices.clone();
                    let interval_config_data = config_data.clone();
                    let interval_error = error.clone();
                    
                    let interval = Interval::new(5000, move || {
                        let devices_inner = interval_devices.clone();
                        let config_data_inner = interval_config_data.clone();
                        let error_inner = interval_error.clone();
                        spawn_local(async move {
                            // Refresh both devices and config to ensure status colors are up to date
                            let devices_result = fetch_devices().await;
                            let config_result = fetch_config_data().await;
                            
                            match devices_result {
                                Ok(device_list) => {
                                    devices_inner.set(device_list);
                                    error_inner.set(None);
                                }
                                Err(e) => {
                                    error_inner.set(Some(e));
                                }
                            }
                            
                            if let Ok(config) = config_result {
                                config_data_inner.set(Some(config));
                            }
                        });
                    });
                    Some(interval)
                } else {
                    None
                };

                // Cleanup function
                move || {
                    drop(interval_handle);
                }
            },
            auto_refresh_val,
        );
    }

    // Fetch user data on component mount
    {
        let user_data = user_data.clone();
        let token_clone = token.clone(); // Clone token for the async block

        use_effect_with_deps(
            move |_| {
                if let Some(auth_token) = token_clone {
                    let user_data_setter = user_data.clone();
                    spawn_local(async move {
                        match fetch_user_data(&auth_token).await {
                            Ok(data) => {
                                user_data_setter.set(Some(data));
                            }
                            Err(e) => {
                                // Handle error fetching user data, e.g., log it or show a message
                                gloo_console::error!("Failed to fetch user data:", e);
                                user_data_setter.set(None); // Or set to a default/guest user
                            }
                        }
                    });
                }
                || () // Cleanup function (no-op here)
            },
            (), // Empty dependency array, so it runs once on mount
        );
    }



    // Fetch topics when a device is selected or auto-refresh is enabled
    {
        let selected_device_id = selected_device.clone();
        let topics = topics.clone();
        let auto_refresh_val = *auto_refresh;

        use_effect_with_deps(
            move |deps: &(Option<String>, bool)| {
                let (device_id, auto_refresh_enabled) = deps.clone();
                
                let interval_handle = if let Some(device_id) = &device_id {
                    let device_id = device_id.clone();
                    let topics_initial = topics.clone();
                    
                    // Initial fetch
                    {
                        let device_id_fetch = device_id.clone();
                        spawn_local(async move {
                            if let Ok(topic_list) = fetch_topics(&device_id_fetch).await {
                                topics_initial.set(topic_list);
                            }
                        });
                    }

                    // Set up auto-refresh for topics if enabled
                    if auto_refresh_enabled {
                        let device_id_interval = device_id.clone();
                        let topics_interval = topics.clone();
                        
                        let interval = Interval::new(5000, move || {
                            let device_id_inner = device_id_interval.clone();
                            let topics_inner = topics_interval.clone();
                            spawn_local(async move {
                                if let Ok(topic_list) = fetch_topics(&device_id_inner).await {
                                    topics_inner.set(topic_list);
                                }
                            });
                        });
                        Some(interval)
                    } else {
                        None
                    }
                } else {
                    topics.set(Vec::new());
                    None
                };

                // Cleanup function
                move || {
                    drop(interval_handle);
                }
            },
            ((*selected_device_id).clone(), auto_refresh_val),
        );
    }

    let on_device_select = {
        let selected_device = selected_device.clone();
        Callback::from(move |device_id: String| {
            selected_device.set(Some(device_id));
        })
    };

    let toggle_auto_refresh = {
        let auto_refresh = auto_refresh.clone();
        Callback::from(move |_| {
            auto_refresh.set(!*auto_refresh);
        })
    };

    let logout = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            LocalStorage::delete("pulson_token");
            navigator.push(&crate::Route::Login);
        })
    };

    let toggle_user_menu = {
        let user_menu_visible = user_menu_visible.clone();
        Callback::from(move |_| {
            user_menu_visible.set(!*user_menu_visible);
        })
    };

    // Callback to toggle selected topic
    let on_topic_select = {
        let selected_topic = selected_topic.clone();
        Callback::from(move |topic_name: String| {
            let current_selected = (*selected_topic).clone();
            if current_selected == Some(topic_name.clone()) {
                selected_topic.set(None); // Deselect if already selected
            } else {
                selected_topic.set(Some(topic_name));
            }
        })
    };

    html! {
        <div class="dashboard-container">
            <aside class="sidebar">
                <div class="sidebar-header">
                    <img src="/static/logo.svg" alt="Pulson Logo" class="nav-logo" />
                    <h1>{"pulson"}</h1>
                </div>

                <nav class="sidebar-nav">
                    <h2>{"Devices"}</h2>
                    if *loading {
                        <div class="loading">{"Loading devices..."}</div>
                    } else if let Some(err) = &*error {
                        <div class="error">{format!("Error: {}", err)}</div>
                    } else if devices.is_empty() {
                        <div class="device-list-empty">
                            <p>{"No devices found"}</p>
                            <small>{"Devices will appear here once they start sending pings"}</small>
                        </div>
                    } else {
                        <div class="device-list">
                            {for {
                                let mut sorted_devices = devices.iter().collect::<Vec<_>>();
                                sorted_devices.sort_by(|a, b| {
                                    let a_time = parse_timestamp(&a.last_seen).unwrap_or(0.0);
                                    let b_time = parse_timestamp(&b.last_seen).unwrap_or(0.0);
                                    b_time.partial_cmp(&a_time).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                sorted_devices
                            }.iter().map(|device| {
                                let device_id = device.device_id.clone();
                                let is_selected = selected_device.as_ref() == Some(&device.device_id);
                                let on_click = {
                                    let device_id = device_id.clone();
                                    let on_select = on_device_select.clone();
                                    Callback::from(move |_| {
                                        on_select.emit(device_id.clone());
                                    })
                                };
                                let status_class = get_device_status_class(&device.status);
                                html! {
                                    <div
                                        class={classes!("device-item", is_selected.then(|| "selected"), status_class)}
                                        onclick={on_click}
                                    >
                                        <div class="device-header">
                                            <span class="device-id">{&device.device_id}</span>
                                            <span class={classes!("device-status", status_class)}>
                                                // {get_device_status(&device.last_seen)} // Removed text
                                            </span>
                                        </div>
                                        <div class="device-info">
                                            <small class="last-seen">
                                                {"Last seen: "}{format_relative_time(&device.last_seen)}
                                            </small>
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    }
                </nav>

                // Configuration section
                <div class="config-section">
                    <h3>{"Thresholds"}</h3>
                    if let Some(config) = &*config_data {
                        <div class="config-info">
                            <div class="config-item">
                                <span class="config-label">{"Online:"}</span>
                                <span class="config-value">{format!("{}s", config.online_threshold_seconds)}</span>
                            </div>
                            <div class="config-item">
                                <span class="config-label">{"Warning:"}</span>
                                <span class="config-value">{format!("{}s", config.warning_threshold_seconds)}</span>
                            </div>
                            <div class="config-item">
                                <span class="config-label">{"Stale:"}</span>
                                <span class="config-value">{format!("{}s", config.stale_threshold_seconds)}</span>
                            </div>
                        </div>
                    } else {
                        <div class="config-loading">
                            <small>{"Loading configuration..."}</small>
                        </div>
                    }
                </div>

                <div class="sidebar-footer">
                    <div class="user-info-container">
                        <div class="user-menu-toggle" onclick={toggle_user_menu.clone()}>
                            <div class="profile-image-placeholder"></div>
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
                            <span class="user-menu-arrow">{ if *user_menu_visible { "▲" } else { "▼" } }</span>
                        </div>
                        if *user_menu_visible {
                            <div class="user-menu-popup">
                                <div class="user-menu-popup-item auto-refresh-control">
                                    <span>{"Autoupdate"}</span>
                                    <button
                                        class={classes!("pill-switch", (*auto_refresh).then(|| "active"))}
                                        onclick={toggle_auto_refresh.clone()}
                                        title={if *auto_refresh { "Auto-Refresh ON" } else { "Auto-Refresh OFF" }}
                                    >
                                        <span class="pill-switch-handle"></span>
                                    </button>
                                </div>
                                <button class="user-menu-popup-item" onclick={logout.clone()}>
                                    {"Logout"}
                                </button>
                                <button class="user-menu-popup-item unimplemented">
                                    {"Settings"}
                                    <small>{" (coming soon)"}</small>
                                </button>
                            </div>
                        }
                    </div>
                </div>
            </aside>

            <main class="main-content"> // Changed from "dashboard-content"
                <section class="topics-panel">
                    <h2>
                        if selected_device.is_some() {
                            {"Topics"}
                        } else {
                            {"Select a device to view topics"}
                        }
                    </h2>
                    if selected_device.is_some() {
                        if topics.is_empty() {
                            <div class="topic-list-empty">
                                <p>{"No topics found"}</p>
                                <small>{"Topics will appear here once the device sends pings"}</small>
                            </div>
                        } else {
                            <div class="topic-list">
                                {for topics.iter().map(|topic| {
                                    let status_class = get_topic_status_class(&topic.status);
                                    let topic_name = topic.topic.clone();
                                    let is_topic_selected = selected_topic.as_ref() == Some(&topic_name);
                                    let on_click_topic = {
                                        let topic_name = topic_name.clone();
                                        let on_topic_select = on_topic_select.clone();
                                        Callback::from(move |_| {
                                            on_topic_select.emit(topic_name.clone());
                                        })
                                    };
                                    html! {
                                        <div class={classes!("topic-item", status_class, is_topic_selected.then(|| "selected"))} onclick={on_click_topic}>
                                            <div class="topic-main-row"> // New wrapper for the main row content
                                                <span class={classes!("topic-status", status_class)}>
                                                    // Status indicator dot
                                                </span>
                                                <div class="topic-content">
                                                    <div class="topic-header">
                                                        <span class="topic-name">{&topic.topic}</span>
                                                    </div>
                                                    <div class="topic-info">
                                                        <small class="last-seen">
                                                            {"Last ping: "}{format_relative_time(&topic.last_seen)}
                                                        </small>
                                                        <small class="exact-time">
                                                            {format_exact_time(&topic.last_seen)}
                                                        </small>
                                                    </div>
                                                </div>
                                            </div> // End of topic-main-row
                                            if is_topic_selected {
                                                <div class="topic-details">
                                                    <h4>{"Last 12 Hours Activity (Placeholder)"}</h4>
                                                    <div class="placeholder-graph">
                                                        <p>{"Graph showing on/off periods will be here."}</p>
                                                        <p>{"[||||| | | ||||||||| | |||| | |||||||]"}</p>
                                                    </div>
                                                    <h4>{"Other Statistics (Placeholder)"}</h4>
                                                    <ul>
                                                        <li>{"Total Pings: N/A"}</li>
                                                        <li>{"Average Uptime: N/A"}</li>
                                                        <li>{"Last Downtime: N/A"}</li>
                                                    </ul>
                                                </div>
                                            }
                                        </div>
                                    }
                                })}
                            </div>
                        }
                    } else {
                        <div class="topic-list-placeholder">
                            <p>{"Select a device to view its topics"}</p>
                        </div>
                    }
                </section>
            </main>
        </div>
    }
}

async fn fetch_user_data(token: &str) -> Result<UserData, String> {
    let request = Request::get("/api/user")
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
    let request = Request::get("/api/config")
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

async fn fetch_devices() -> Result<Vec<DeviceInfo>, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let request = Request::get("/api/devices")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<Vec<DeviceInfo>>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}

async fn fetch_topics(device_id: &str) -> Result<Vec<TopicInfo>, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let request = Request::get(&format!("/api/devices/{}", device_id))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<Vec<TopicInfo>>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}

fn get_device_status_class(status: &str) -> &'static str {
    match status {
        "Online" => "online",
        "Warning" => "warning", 
        "Offline" => "offline",
        _ => "unknown",
    }
}

fn get_topic_status_class(status: &str) -> &'static str {
    match status {
        "Active" => "online",
        "Recent" => "warning",
        "Stale" => "offline", 
        "Inactive" => "offline",
        _ => "unknown",
    }
}

fn format_relative_time(timestamp: &str) -> String {
    if let Ok(ts) = parse_timestamp(timestamp) {
        let now = Date::now();
        let diff_ms = now - ts;
        let diff_seconds = (diff_ms / 1000.0) as i64;

        if diff_seconds < 1 {
            "just now".to_string()
        } else if diff_seconds < 60 {
            format!("{}s ago", diff_seconds)
        } else if diff_seconds < 3600 {
            format!("{}m ago", diff_seconds / 60)
        } else if diff_seconds < 86400 {
            format!("{}h ago", diff_seconds / 3600)
        } else {
            format!("{}d ago", diff_seconds / 86400)
        }
    } else {
        "unknown".to_string()
    }
}

fn format_exact_time(timestamp: &str) -> String {
    if let Ok(ts) = parse_timestamp(timestamp) {
        let date = Date::new(&ts.into());
        date.to_iso_string()
            .as_string()
            .unwrap_or_else(|| timestamp.to_string())
    } else {
        timestamp.to_string()
    }
}

fn parse_timestamp(timestamp: &str) -> Result<f64, ()> {
    let js_value = Date::parse(timestamp);
    if js_value.is_nan() {
        Err(())
    } else {
        Ok(js_value)
    }
}
