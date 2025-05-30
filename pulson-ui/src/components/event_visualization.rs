use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use serde::{Deserialize};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Clone, PartialEq, Deserialize)]
pub struct EventHistoryData {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub data: Vec<Value>,
}

#[derive(Clone, PartialEq)]
pub struct EventEntry {
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub category: EventCategory,
}

#[derive(Clone, PartialEq)]
pub enum EventCategory {
    Info,
    Warning,
    Error,
    Success,
    Debug,
}

#[derive(Properties, Clone, PartialEq)]
pub struct EventVisualizationProps {
    pub device_id: String,
    pub topic: String,
}

#[function_component(EventVisualization)]
pub fn event_visualization(props: &EventVisualizationProps) -> Html {
    let event_history = use_state(|| None::<EventHistoryData>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let refresh_interval = use_state(|| None::<Interval>);
    let selected_category = use_state(|| None::<EventCategory>);
    let show_timestamps = use_state(|| true);

    // Auto-refresh function
    let refresh_data = {
        let device_id = props.device_id.clone();
        let topic = props.topic.clone();
        let event_history = event_history.clone();
        let error = error.clone();
        
        Callback::from(move |_| {
            let device_id = device_id.clone();
            let topic = topic.clone();
            let event_history = event_history.clone();
            let error = error.clone();

            spawn_local(async move {
                error.set(None);

                match fetch_event_history(&device_id, &topic).await {
                    Ok(history) => {
                        event_history.set(Some(history));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch event data: {}", e)));
                    }
                }
            });
        })
    };

    // Manual refresh callback for button clicks
    let manual_refresh = {
        let refresh_data = refresh_data.clone();
        Callback::from(move |_: web_sys::MouseEvent| {
            refresh_data.emit(());
        })
    };

    // Set up auto-refresh interval
    {
        let refresh_data = refresh_data.clone();
        let refresh_interval = refresh_interval.clone();
        
        use_effect_with_deps(
            move |_| {
                refresh_interval.set(None);
                
                let interval = Interval::new(5000, move || {
                    refresh_data.emit(());
                });
                
                refresh_interval.set(Some(interval));
                
                || {}
            },
            (),
        );
    }

    // Initial fetch when device or topic changes
    {
        let refresh_data = refresh_data.clone();
        let loading = loading.clone();

        use_effect_with_deps(
            move |_| {
                loading.set(true);
                refresh_data.emit(());
                loading.set(false);
                || {}
            },
            (props.device_id.clone(), props.topic.clone()),
        );
    }

    // Parse event entries
    let events = event_history.as_ref()
        .map(|history| parse_event_entries(&history.data))
        .unwrap_or_default();

    // Filter events by category if selected
    let filtered_events: Vec<_> = if let Some(ref category) = *selected_category {
        events.iter().filter(|e| &e.category == category).collect()
    } else {
        events.iter().collect()
    };

    // Toggle category filter
    let on_category_filter = {
        let selected_category = selected_category.clone();
        Callback::from(move |category: Option<EventCategory>| {
            selected_category.set(category);
        })
    };

    // Toggle timestamp display
    let toggle_timestamps = {
        let show_timestamps = show_timestamps.clone();
        Callback::from(move |_| {
            show_timestamps.set(!*show_timestamps);
        })
    };

    html! {
        <div class="event-visualization">
            <div class="event-viz-header">
                <h3>{"Event Log: "}{&props.topic}</h3>
                <div class="event-controls">
                    <button 
                        onclick={toggle_timestamps} 
                        class={classes!("control-btn", (*show_timestamps).then(|| "active"))}
                        title="Toggle timestamps"
                    >
                        {"🕒"}
                    </button>
                    <button onclick={manual_refresh} class="refresh-btn">{"Refresh"}</button>
                </div>
            </div>

            if *loading {
                <div class="event-loading">{"Loading event data..."}</div>
            } else if let Some(error_msg) = (*error).as_ref() {
                <div class="event-error">{error_msg}</div>
            } else if events.is_empty() {
                <div class="event-empty">
                    <p>{"No events available for this topic"}</p>
                    <small>{"Event messages will appear here when data is received"}</small>
                </div>
            } else {
                <div class="event-content">
                    // Category filters
                    <div class="category-filters">
                        <button
                            class={classes!("category-btn", "all", (*selected_category).is_none().then(|| "active"))}
                            onclick={
                                let on_category_filter = on_category_filter.clone();
                                Callback::from(move |_| on_category_filter.emit(None))
                            }
                        >
                            {"All"} <span class="count">{"("}{events.len()}{")"}</span>
                        </button>
                        
                        {for [
                            (EventCategory::Info, "Info", "🔵"),
                            (EventCategory::Success, "Success", "🟢"),
                            (EventCategory::Warning, "Warning", "🟡"),
                            (EventCategory::Error, "Error", "🔴"),
                            (EventCategory::Debug, "Debug", "⚪"),
                        ].iter().map(|(cat, label, icon)| {
                            let count = events.iter().filter(|e| &e.category == cat).count();
                            let is_selected = selected_category.as_ref() == Some(cat);
                            let category_clone = cat.clone();
                            
                            html! {
                                <button
                                    class={classes!("category-btn", get_category_class(cat), is_selected.then(|| "active"))}
                                    onclick={
                                        let on_category_filter = on_category_filter.clone();
                                        Callback::from(move |_| on_category_filter.emit(Some(category_clone.clone())))
                                    }
                                >
                                    {icon} {" "} {label} <span class="count">{"("}{count}{")"}</span>
                                </button>
                            }
                        })}
                    </div>

                    // Event statistics
                    <div class="event-stats">
                        <div class="stat-item">
                            <span class="stat-label">{"Total Events:"}</span>
                            <span class="stat-value">{events.len()}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Showing:"}</span>
                            <span class="stat-value">{filtered_events.len()}</span>
                        </div>
                        if let Some(latest) = events.first() {
                            <div class="stat-item">
                                <span class="stat-label">{"Latest:"}</span>
                                <span class="stat-value">{format_time_ago(&latest.timestamp)}</span>
                            </div>
                        }
                    </div>

                    // Event list
                    <div class="event-list">
                        {for filtered_events.iter().enumerate().map(|(_index, event)| {
                            html! {
                                <div class={classes!("event-entry", get_category_class(&event.category))}>
                                    <div class="event-header">
                                        <span class="event-icon">{get_category_icon(&event.category)}</span>
                                        if *show_timestamps {
                                            <span class="event-timestamp">
                                                {event.timestamp.format("%H:%M:%S").to_string()}
                                            </span>
                                        }
                                        <span class="event-time-ago">
                                            {format_time_ago(&event.timestamp)}
                                        </span>
                                    </div>
                                    <div class="event-message">
                                        {&event.message}
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                </div>
            }
        </div>
    }
}

// Helper functions

fn parse_event_entries(data: &[Value]) -> Vec<EventEntry> {
    let mut events: Vec<EventEntry> = data
        .iter()
        .filter_map(|entry| {
            let timestamp_str = entry["timestamp"].as_str()?;
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str).ok()?.with_timezone(&Utc);
            
            if let Some(event_data) = entry["data"].as_object() {
                if let Some(event_obj) = event_data.get("Event").and_then(|e| e.as_object()) {
                    let message = event_obj.get("message").and_then(|m| m.as_str())?.to_string();
                    let category = categorize_message(&message);
                    
                    return Some(EventEntry {
                        message,
                        timestamp,
                        category,
                    });
                }
            }
            
            None
        })
        .collect();

    // Sort by timestamp (newest first)
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    events
}

fn categorize_message(message: &str) -> EventCategory {
    let message_lower = message.to_lowercase();
    
    if message_lower.contains("error") || message_lower.contains("fail") || message_lower.contains("critical") {
        EventCategory::Error
    } else if message_lower.contains("warn") || message_lower.contains("caution") || message_lower.contains("alert") {
        EventCategory::Warning
    } else if message_lower.contains("success") || message_lower.contains("complete") || message_lower.contains("ok") {
        EventCategory::Success
    } else if message_lower.contains("debug") || message_lower.contains("trace") || message_lower.starts_with("[debug]") {
        EventCategory::Debug
    } else {
        EventCategory::Info
    }
}

fn get_category_class(category: &EventCategory) -> &'static str {
    match category {
        EventCategory::Info => "info",
        EventCategory::Warning => "warning",
        EventCategory::Error => "error",
        EventCategory::Success => "success",
        EventCategory::Debug => "debug",
    }
}

fn get_category_icon(category: &EventCategory) -> &'static str {
    match category {
        EventCategory::Info => "🔵",
        EventCategory::Warning => "🟡",
        EventCategory::Error => "🔴",
        EventCategory::Success => "🟢",
        EventCategory::Debug => "⚪",
    }
}

fn format_time_ago(timestamp: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*timestamp);
    
    if duration.num_seconds() < 60 {
        format!("{}s ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}

// API call to fetch event history
async fn fetch_event_history(device_id: &str, topic: &str) -> Result<EventHistoryData, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let url = format!("/api/devices/{}/data?topic={}&type=event", device_id, topic);

    let request = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        let response: Value = request
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Transform the data response to match our expected format
        let data = response.get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(EventHistoryData {
            time_range: "1d".to_string(),
            start_time: "".to_string(),
            end_time: "".to_string(),
            data,
        })
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}
