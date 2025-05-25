use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::Deserialize;
use wasm_bindgen_futures::spawn_local;
use js_sys::Date;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, PartialEq, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub last_seen: String,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub last_seen: String,
}

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let devices = use_state(Vec::<DeviceInfo>::new);
    let selected_device = use_state(|| None::<String>);
    let topics = use_state(Vec::<TopicInfo>::new);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let auto_refresh = use_state(|| true);
    let navigator = use_navigator().unwrap();

    // Check if user is authenticated
    let token = LocalStorage::get::<String>("pulson_token").ok();
    if token.is_none() {
        navigator.push(&crate::Route::Login);
        return html! {};
    }

    // Fetch devices on component mount and when auto_refresh changes
    {
        let devices = devices.clone();
        let loading = loading.clone();
        let error = error.clone();
        let auto_refresh_val = *auto_refresh;

        use_effect_with_deps(
            move |_| {
                let devices_initial = devices.clone();
                let loading_initial = loading.clone();
                let error_initial = error.clone();

                spawn_local(async move {
                    loading_initial.set(true);
                    match fetch_devices().await {
                        Ok(device_list) => {
                            devices_initial.set(device_list);
                            error_initial.set(None);
                        }
                        Err(e) => {
                            error_initial.set(Some(e));
                        }
                    }
                    loading_initial.set(false);
                });

                // Set up auto-refresh if enabled
                if auto_refresh_val {
                    // Clone for the interval closure
                    let interval_devices = devices.clone();
                    let interval_error = error.clone();
                    let interval = gloo_timers::callback::Interval::new(5000, move || {
                        let devices_inner = interval_devices.clone();
                        let error_inner = interval_error.clone();
                        spawn_local(async move {
                            if let Ok(device_list) = fetch_devices().await {
                                devices_inner.set(device_list);
                                error_inner.set(None);
                            }
                        });
                    });
                    interval.forget();
                }
                || ()
            },
            auto_refresh_val,
        );
    }

    // Fetch topics when a device is selected
    {
        let selected_device_id = selected_device.clone();
        let topics = topics.clone();

        use_effect_with_deps(
            move |device_id: &Option<String>| {
                if let Some(device_id) = device_id {
                    let device_id = device_id.clone();
                    let topics = topics.clone();
                    spawn_local(async move {
                        if let Ok(topic_list) = fetch_topics(&device_id).await {
                            topics.set(topic_list);
                        }
                    });
                } else {
                    topics.set(Vec::new());
                }
                || ()
            },
            (*selected_device_id).clone(),
        );
    }

    let on_device_select = {
        let selected_device = selected_device.clone();
        Callback::from(move |device_id: String| {
            selected_device.set(Some(device_id));
        })
    };

    let on_refresh = {
        let devices = devices.clone();
        let loading = loading.clone();
        let error = error.clone();
        Callback::from(move |_| {
            let devices = devices.clone();
            let loading = loading.clone();
            let error = error.clone();
            spawn_local(async move {
                loading.set(true);
                match fetch_devices().await {
                    Ok(device_list) => {
                        devices.set(device_list);
                        error.set(None);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                loading.set(false);
            });
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

    html! {
        <div class="dashboard">
            <nav class="navigation">
                <div class="nav-brand">
                    <h1>{"Pulson"}</h1>
                    <span class="nav-subtitle">{"System Monitor"}</span>
                </div>
                
                <div class="nav-controls">
                    <button 
                        class="nav-button refresh-button"
                        onclick={on_refresh}
                        title="Refresh data"
                    >
                        {"🔄"}
                    </button>
                    
                    <button 
                        class={classes!("nav-button", "auto-refresh-button", (*auto_refresh).then(|| "active"))}
                        onclick={toggle_auto_refresh}
                        title={if *auto_refresh { "Disable auto-refresh" } else { "Enable auto-refresh" }}
                    >
                        if *auto_refresh {
                            {"⏸️ Auto"}
                        } else {
                            {"▶️ Auto"}
                        }
                    </button>
                    
                    <button 
                        class="nav-button logout-button"
                        onclick={logout}
                        title="Logout"
                    >
                        {"Logout"}
                    </button>
                </div>
            </nav>
            
            <main class="dashboard-content">
                <div class="dashboard-grid">
                    <section class="devices-panel">
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
                                {for devices.iter().map(|device| {
                                    let device_id = device.device_id.clone();
                                    let is_selected = selected_device.as_ref() == Some(&device.device_id);
                                    let on_click = {
                                        let device_id = device_id.clone();
                                        let on_select = on_device_select.clone();
                                        Callback::from(move |_| {
                                            on_select.emit(device_id.clone());
                                        })
                                    };

                                    let status_class = get_device_status_class(&device.last_seen);
                                    
                                    html! {
                                        <div 
                                            class={classes!("device-item", is_selected.then(|| "selected"), status_class)}
                                            onclick={on_click}
                                        >
                                            <div class="device-header">
                                                <span class="device-id">{&device.device_id}</span>
                                                <span class={classes!("device-status", status_class)}>
                                                    {get_device_status(&device.last_seen)}
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
                    </section>

                    <section class="topics-panel">
                        <h2>
                            if let Some(device_id) = &*selected_device {
                                {format!("Topics for {}", device_id)}
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
                                        let status_class = get_topic_status_class(&topic.last_seen);
                                        html! {
                                            <div class={classes!("topic-item", status_class)}>
                                                <div class="topic-header">
                                                    <span class="topic-name">{&topic.topic}</span>
                                                    <span class={classes!("topic-status", status_class)}>
                                                        {get_topic_status(&topic.last_seen)}
                                                    </span>
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

                    <section class="ping-panel">
                        <h2>{"Send Ping"}</h2>
                        <div class="ping-form">
                            // Simple ping form inline for now
                            <p>{"Ping functionality will be added here"}</p>
                        </div>
                    </section>

                    <section class="stats-panel">
                        <h2>{"Statistics"}</h2>
                        <div class="stats-grid">
                            <div class="stat-card">
                                <h3>{"Total Devices"}</h3>
                                <span class="stat-value">{devices.len()}</span>
                            </div>
                            <div class="stat-card">
                                <h3>{"Active Topics"}</h3>
                                <span class="stat-value">{topics.len()}</span>
                            </div>
                            <div class="stat-card">
                                <h3>{"Auto Refresh"}</h3>
                                <span class="stat-value">{if *auto_refresh { "ON" } else { "OFF" }}</span>
                            </div>
                        </div>
                    </section>
                </div>
            </main>
        </div>
    }
}

async fn fetch_devices() -> Result<Vec<DeviceInfo>, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let request = Request::get("/devices")
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

    let request = Request::get(&format!("/devices/{}", device_id))
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

fn get_device_status_class(last_seen: &str) -> &'static str {
    if let Ok(timestamp) = parse_timestamp(last_seen) {
        let now = Date::now();
        let diff_ms = now - timestamp;
        let diff_seconds = diff_ms / 1000.0;
        
        if diff_seconds < 30.0 {
            "online"
        } else if diff_seconds < 300.0 { // 5 minutes
            "warning"
        } else {
            "offline"
        }
    } else {
        "unknown"
    }
}

fn get_device_status(last_seen: &str) -> &'static str {
    match get_device_status_class(last_seen) {
        "online" => "●",
        "warning" => "⚠",
        "offline" => "●",
        _ => "?",
    }
}

fn get_topic_status_class(last_seen: &str) -> &'static str {
    if let Ok(timestamp) = parse_timestamp(last_seen) {
        let now = Date::now();
        let diff_ms = now - timestamp;
        let diff_seconds = diff_ms / 1000.0;
        
        if diff_seconds < 30.0 {
            "active"
        } else if diff_seconds < 300.0 { // 5 minutes
            "recent"
        } else if diff_seconds < 3600.0 { // 1 hour
            "stale"
        } else {
            "inactive"
        }
    } else {
        "unknown"
    }
}

fn get_topic_status(last_seen: &str) -> &'static str {
    match get_topic_status_class(last_seen) {
        "active" => "🟢",
        "recent" => "🟡",
        "stale" => "🟠",
        "inactive" => "🔴",
        _ => "⚪",
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
        date.to_iso_string().as_string().unwrap_or_else(|| timestamp.to_string())
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
