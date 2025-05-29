use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use js_sys::Date;
use serde::Deserialize;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;
use super::pulse_visualization::PulseVisualization;
use super::inline_map::InlineMap;
use super::image_visualization::ImageVisualization;
use super::sensor_visualization::SensorVisualization;
use super::event_visualization::EventVisualization;
use super::trigger_visualization::TriggerVisualization;

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
    pub data_type: String, // Single data type: ping, event, value, array, or bytes
}

#[derive(Clone, PartialEq, Deserialize, Debug)] // Added Debug for easier inspection
pub struct UserData {
    pub username: String,
    pub is_root: bool,
}

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let devices = use_state(Vec::<DeviceInfo>::new);
    let selected_device = use_state(|| None::<String>);
    let topics = use_state(Vec::<TopicInfo>::new);
    let selected_topic = use_state(|| None::<String>);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let auto_refresh = use_state(|| true);
    let navigator = use_navigator().unwrap();
    let user_menu_visible = use_state(|| false);
    let user_data = use_state(|| None::<UserData>);
    let mobile_menu_visible = use_state(|| false);
    
    let main_content_ref = use_node_ref();

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

                // Initial fetch
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

                // Set up auto-refresh interval if enabled
                let interval_handle = if auto_refresh_val {
                    let interval_devices = devices.clone();
                    let interval_error = error.clone();
                    
                    let interval = Interval::new(5000, move || {
                        let devices_inner = interval_devices.clone();
                        let error_inner = interval_error.clone();
                        spawn_local(async move {
                            match fetch_devices().await {
                                Ok(device_list) => {
                                    devices_inner.set(device_list);
                                    error_inner.set(None);
                                }
                                Err(e) => {
                                    error_inner.set(Some(e));
                                }
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
        let mobile_menu_visible = mobile_menu_visible.clone();
        Callback::from(move |device_id: String| {
            selected_device.set(Some(device_id));
            // Close mobile menu when device is selected
            mobile_menu_visible.set(false);
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

    let toggle_mobile_menu = {
        let mobile_menu_visible = mobile_menu_visible.clone();
        Callback::from(move |_| {
            mobile_menu_visible.set(!*mobile_menu_visible);
        })
    };

    let close_mobile_menu = {
        let mobile_menu_visible = mobile_menu_visible.clone();
        Callback::from(move |_| {
            mobile_menu_visible.set(false);
        })
    };

    let go_to_settings = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            navigator.push(&crate::Route::Settings);
        })
    };

    let on_topic_select = {
        let selected_topic_handle = selected_topic.clone();
        Callback::from(move |topic_name: String| {
            let current_selected = (*selected_topic_handle).clone();
            if current_selected == Some(topic_name.clone()) {
                selected_topic_handle.set(None); // Deselect if already selected
            } else {
                selected_topic_handle.set(Some(topic_name));
            }
        })
    };

    html! {
        <div class="dashboard-container">
            // Mobile overlay
            if *mobile_menu_visible {
                <div class="mobile-overlay visible" onclick={close_mobile_menu.clone()}></div>
            }
            
            <aside class={classes!("sidebar", (*mobile_menu_visible).then(|| "mobile-open"))}>
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
                                <button 
                                    class="user-menu-popup-item" 
                                    onclick={go_to_settings}
                                >
                                    {"Settings"}
                                </button>
                            </div>
                        }
                    </div>
                </div>
            </aside>

            <main class="main-content" ref={main_content_ref}>
                // Mobile header with logo and burger menu
                <div class="mobile-header">
                    <button class="mobile-menu-toggle" onclick={toggle_mobile_menu.clone()}>
                        <div class="hamburger-icon">
                            <div class="hamburger-line"></div>
                            <div class="hamburger-line"></div>
                            <div class="hamburger-line"></div>
                        </div>
                    </button>
                    <img src="/static/logo.svg" alt="Pulson Logo" class="nav-logo" />
                    <h1 class="brand-text">{"pulson"}</h1>
                </div>
                                
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
                                    let device_id_for_pulse = selected_device.as_ref().unwrap().clone();
                                    let _current_topic_status = topic.status.clone(); // Get the status for the current topic
                                    let on_click_topic = {
                                        let topic_name = topic_name.clone();
                                        let on_topic_select = on_topic_select.clone();
                                        Callback::from(move |_| {
                                            on_topic_select.emit(topic_name.clone());
                                        })
                                    };
                                    html! {
                                        <div class={classes!("topic-item", status_class, is_topic_selected.then(|| "selected"))}>
                                            <div class="topic-main-row" onclick={on_click_topic}> // New wrapper for the main row content
                                                <span class={classes!("topic-status", status_class)}>
                                                    // Status indicator dot
                                                </span>
                                                <div class="topic-content">
                                                    <div class="topic-header">
                                                        <span class="topic-name">{&topic.topic}</span>
                                                        <div class="data-type-labels">
                                                            // Display single data type
                                                            <span class="data-type-label data-type" title={format!("Data Type: {}", &topic.data_type)}>{&topic.data_type}</span>
                                                        </div>
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
                                                    if topic.data_type == "pulse" {
                                                        <PulseVisualization
                                                            device_id={device_id_for_pulse}
                                                            topic={Some(topic_name.clone())}
                                                        />
                                                    } else if topic.data_type == "gps" {
                                                        <InlineMap
                                                            device_id={device_id_for_pulse}
                                                            topic={topic_name.clone()}
                                                        />
                                                    } else if topic.data_type == "sensor" {
                                                        <SensorVisualization
                                                            device_id={device_id_for_pulse}
                                                            topic={topic_name.clone()}
                                                        />
                                                    } else if topic.data_type == "trigger" {
                                                        <TriggerVisualization
                                                            device_id={device_id_for_pulse}
                                                            topic={topic_name.clone()}
                                                        />
                                                    } else if topic.data_type == "event" {
                                                        <EventVisualization
                                                            device_id={device_id_for_pulse}
                                                            topic={topic_name.clone()}
                                                        />
                                                    } else if topic.data_type == "image" {
                                                        <ImageVisualization
                                                            device_id={device_id_for_pulse}
                                                            topic={topic_name.clone()}
                                                        />
                                                    } else {
                                                        <div class="unimplemented-message">
                                                            <h4>{"Data Type: "}{&topic.data_type.to_uppercase()}</h4>
                                                            <p class="unimplemented-text">{"Visualization for this data type is not yet implemented"}</p>
                                                            <small class="unimplemented-hint">{"Supported types: pulse, gps, sensor, trigger, event, image"}</small>
                                                        </div>
                                                    }
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
        "Active" => "online", // Maps to .online CSS class
        "Recent" => "warning",  // Maps to .warning CSS class
        "Stale" => "offline",  // Maps to .offline CSS class
        "Inactive" => "offline", // Maps to .offline CSS class (or a new .inactive if specific styling needed)
        _ => "unknown",
    }
}

// Helper function to format relative time
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
    // Try to parse as ISO 8601 format first
    if let Ok(parsed_time) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        // Format as European: DD/MM/YYYY HH:MM:SS
        parsed_time.format("%d/%m/%Y %H:%M:%S").to_string()
    } else if let Ok(ts) = parse_timestamp(timestamp) {
        // Fallback to JS Date parsing and manual formatting
        let date = Date::new(&ts.into());
        let day = date.get_utc_date();
        let month = date.get_utc_month() + 1; // JS months are 0-based
        let year = date.get_utc_full_year();
        let hours = date.get_utc_hours();
        let minutes = date.get_utc_minutes();
        let seconds = date.get_utc_seconds();
        format!("{:02}/{:02}/{} {:02}:{:02}:{:02}", day, month, year, hours, minutes, seconds)
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
