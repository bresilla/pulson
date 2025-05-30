use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use serde::{Deserialize};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Clone, PartialEq, Deserialize)]
pub struct TriggerHistoryData {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub data: Vec<Value>,
}

#[derive(Clone, PartialEq)]
pub struct TriggerState {
    pub state: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct TriggerVisualizationProps {
    pub device_id: String,
    pub topic: String,
}

#[function_component(TriggerVisualization)]
pub fn trigger_visualization(props: &TriggerVisualizationProps) -> Html {
    let trigger_history = use_state(|| None::<TriggerHistoryData>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let refresh_interval = use_state(|| None::<Interval>);

    // Auto-refresh function
    let refresh_data = {
        let device_id = props.device_id.clone();
        let topic = props.topic.clone();
        let trigger_history = trigger_history.clone();
        let error = error.clone();
        
        Callback::from(move |_| {
            let device_id = device_id.clone();
            let topic = topic.clone();
            let trigger_history = trigger_history.clone();
            let error = error.clone();

            spawn_local(async move {
                error.set(None);

                match fetch_trigger_history(&device_id, &topic).await {
                    Ok(history) => {
                        trigger_history.set(Some(history));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch trigger data: {}", e)));
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

    // Parse trigger states
    let trigger_states = trigger_history.as_ref()
        .map(|history| parse_trigger_states(&history.data))
        .unwrap_or_default();

    // Get current state
    let current_state = trigger_states.first().map(|s| s.state);

    // Calculate statistics
    let stats = calculate_trigger_stats(&trigger_states);

    html! {
        <div class="trigger-visualization">
            <div class="trigger-viz-header">
                <h3>{"Digital Trigger: "}{&props.topic}</h3>
                <button onclick={manual_refresh} class="btn">{"Refresh"}</button>
            </div>

            if *loading {
                <div class="trigger-loading">{"Loading trigger data..."}</div>
            } else if let Some(error_msg) = (*error).as_ref() {
                <div class="trigger-error">{error_msg}</div>
            } else if let Some(state) = current_state {
                <div class="trigger-content">
                    // Main State Display
                    <div class="trigger-state-display">
                        <div class={classes!("state-indicator", if state { "active" } else { "inactive" })}>
                            <div class="state-icon">
                                {if state { "🟢" } else { "⚫" }}
                            </div>
                            <div class="state-text">
                                <span class="state-label">{"Current State"}</span>
                                <span class="state-value">
                                    {if state { "ACTIVE" } else { "INACTIVE" }}
                                </span>
                                if let Some(latest) = trigger_states.first() {
                                    <span class="state-time">
                                        {"Since "}{format_time_ago(&latest.timestamp)}
                                    </span>
                                }
                            </div>
                        </div>
                    </div>

                    // State Timeline
                    if !trigger_states.is_empty() {
                        <div class="trigger-timeline">
                            <h4>{"Recent State Changes"}</h4>
                            <div class="timeline-container">
                                <div class="timeline-track">
                                    {for trigger_states.iter().take(10).enumerate().map(|(i, state)| {
                                        let position = (i as f64 / 9.0_f64.max(1.0)) * 100.0;
                                        html! {
                                            <div 
                                                class={classes!("timeline-point", if state.state { "active" } else { "inactive" })}
                                                style={format!("left: {}%", position)}
                                                title={format!("State: {} at {}", 
                                                    if state.state { "Active" } else { "Inactive" },
                                                    state.timestamp.format("%H:%M:%S")
                                                )}
                                            >
                                                <div class="timeline-marker"></div>
                                            </div>
                                        }
                                    })}
                                </div>
                            </div>
                            
                            // State change history
                            <div class="state-history">
                                {for trigger_states.iter().take(5).map(|state| {
                                    html! {
                                        <div class={classes!("history-entry", if state.state { "active" } else { "inactive" })}>
                                            <span class="history-icon">
                                                {if state.state { "🟢" } else { "⚫" }}
                                            </span>
                                            <span class="history-state">
                                                {if state.state { "Active" } else { "Inactive" }}
                                            </span>
                                            <span class="history-time">
                                                {state.timestamp.format("%H:%M:%S").to_string()}
                                            </span>
                                            <span class="history-ago">
                                                {"("}{format_time_ago(&state.timestamp)}{")"}
                                            </span>
                                        </div>
                                    }
                                })}
                            </div>
                        </div>
                    }

                    // Statistics
                    <div class="trigger-stats">
                        <div class="stat-item">
                            <span class="stat-label">{"Total Changes:"}</span>
                            <span class="stat-value">{trigger_states.len()}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Active Time:"}</span>
                            <span class="stat-value">{format!("{:.1}%", stats.active_percentage)}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Toggles/Hour:"}</span>
                            <span class="stat-value">{format!("{:.1}", stats.toggles_per_hour)}</span>
                        </div>
                        if let Some(ref duration) = stats.current_state_duration {
                            <div class="stat-item">
                                <span class="stat-label">{"Current Duration:"}</span>
                                <span class="stat-value">{duration}</span>
                            </div>
                        }
                    </div>
                </div>
            } else {
                <div class="trigger-empty">
                    <p>{"No trigger data available for this topic"}</p>
                    <small>{"Digital trigger states will appear here when data is received"}</small>
                </div>
            }
        </div>
    }
}

// Helper structures and functions

#[derive(Clone, PartialEq)]
struct TriggerStats {
    active_percentage: f64,
    toggles_per_hour: f64,
    current_state_duration: Option<String>,
}

fn parse_trigger_states(data: &[Value]) -> Vec<TriggerState> {
    let mut states: Vec<TriggerState> = data
        .iter()
        .filter_map(|entry| {
            let timestamp_str = entry["timestamp"].as_str()?;
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str).ok()?.with_timezone(&Utc);
            
            if let Some(trigger_data) = entry["data"].as_object() {
                if let Some(trigger_obj) = trigger_data.get("Trigger").and_then(|t| t.as_object()) {
                    let state = trigger_obj.get("state").and_then(|s| s.as_bool())?;
                    
                    return Some(TriggerState {
                        state,
                        timestamp,
                    });
                }
            }
            
            None
        })
        .collect();

    // Sort by timestamp (newest first)
    states.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    states
}

fn calculate_trigger_stats(states: &[TriggerState]) -> TriggerStats {
    if states.is_empty() {
        return TriggerStats {
            active_percentage: 0.0,
            toggles_per_hour: 0.0,
            current_state_duration: None,
        };
    }

    let now = Utc::now();
    let total_duration = if states.len() > 1 {
        states.first().unwrap().timestamp.signed_duration_since(states.last().unwrap().timestamp)
    } else {
        chrono::Duration::hours(1) // Default to 1 hour if only one state
    };

    // Calculate active time percentage
    let mut active_duration = chrono::Duration::zero();
    for window in states.windows(2) {
        if window[1].state { // Previous state was active
            active_duration = active_duration + window[0].timestamp.signed_duration_since(window[1].timestamp);
        }
    }

    // Add current state duration if active
    if let Some(latest) = states.first() {
        if latest.state {
            active_duration = active_duration + now.signed_duration_since(latest.timestamp);
        }
    }

    let active_percentage = if total_duration.num_seconds() > 0 {
        (active_duration.num_seconds() as f64 / total_duration.num_seconds() as f64) * 100.0
    } else {
        0.0
    };

    // Calculate toggles per hour
    let toggles_per_hour = if total_duration.num_hours() > 0 {
        (states.len() - 1) as f64 / total_duration.num_hours() as f64
    } else {
        0.0
    };

    // Current state duration
    let current_state_duration = states.first().map(|latest| {
        let duration = now.signed_duration_since(latest.timestamp);
        format_duration(duration)
    });

    TriggerStats {
        active_percentage,
        toggles_per_hour,
        current_state_duration,
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

fn format_duration(duration: chrono::Duration) -> String {
    if duration.num_seconds() < 60 {
        format!("{}s", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h {}m", duration.num_hours(), duration.num_minutes() % 60)
    } else {
        format!("{}d {}h", duration.num_days(), duration.num_hours() % 24)
    }
}

// API call to fetch trigger history
async fn fetch_trigger_history(device_id: &str, topic: &str) -> Result<TriggerHistoryData, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let url = format!("/api/devices/{}/data?topic={}&type=trigger", device_id, topic);

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

        Ok(TriggerHistoryData {
            time_range: "1d".to_string(),
            start_time: "".to_string(),
            end_time: "".to_string(),
            data,
        })
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}
