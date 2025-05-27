use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq, Deserialize)]
pub struct PulseHistoryData {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub data: Vec<Value>,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct PulseStats {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub stats: Vec<Value>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct PulseVisualizationProps {
    pub device_id: String,
    pub topic: Option<String>,
    pub topic_status: String, // Add topic_status prop
}

#[function_component(PulseVisualization)]
pub fn pulse_visualization(props: &PulseVisualizationProps) -> Html {
    let pulse_history = use_state(|| None::<PulseHistoryData>);
    let pulse_stats = use_state(|| None::<PulseStats>);
    let selected_time_range = use_state(|| "1h".to_string());
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);

    let topic_status_color = match props.topic_status.as_str() {
        "Active" => "var(--status-color-active)",
        "Recent" => "var(--status-color-recent)",
        "Stale" => "var(--status-color-stale)",
        "Inactive" => "var(--status-color-inactive)",
        _ => "var(--accent-color)", // Default or fallback
    };

    // Fetch pulse history when device, topic, or time range changes
    {
        let device_id = props.device_id.clone();
        let topic = props.topic.clone();
        let time_range = (*selected_time_range).clone();
        let pulse_history = pulse_history.clone();
        let pulse_stats = pulse_stats.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with_deps(
            move |_| {
                let device_id = device_id.clone();
                let topic = topic.clone();
                let time_range = time_range.clone();
                let pulse_history = pulse_history.clone();
                let pulse_stats = pulse_stats.clone();
                let loading = loading.clone();
                let error = error.clone();

                spawn_local(async move {
                    loading.set(true);
                    error.set(None);

                    // Fetch pulse history
                    match fetch_pulse_history(&device_id, &time_range, topic.as_deref()).await {
                        Ok(history) => {
                            pulse_history.set(Some(history));
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to fetch pulse history: {}", e)));
                        }
                    }

                    // Fetch pulse stats
                    match fetch_pulse_stats(&device_id, &time_range).await {
                        Ok(stats) => {
                            pulse_stats.set(Some(stats));
                        }
                        Err(e) => {
                            error.set(Some(format!("Failed to fetch pulse stats: {}", e)));
                        }
                    }

                    loading.set(false);
                });

                || {}
            },
            (props.device_id.clone(), props.topic.clone(), (*selected_time_range).clone()),
        );
    }

    // Time range selection callback
    let on_time_range_change = {
        let selected_time_range = selected_time_range.clone();
        Callback::from(move |range: String| {
            selected_time_range.set(range);
        })
    };

    html! {
        <div class="pulse-visualization" style={format!("--pulse-viz-accent-color: {};", topic_status_color)}>
            <div class="pulse-viz-header">
                <h3>{"Pulse History"}</h3>
                <div class="time-range-selector">
                    <button 
                        class={classes!("time-range-btn", (*selected_time_range == "1h").then(|| "active"))}
                        onclick={
                            let callback = on_time_range_change.clone();
                            Callback::from(move |_| callback.emit("1h".to_string()))
                        }
                    >
                        {"1H"}
                    </button>
                    <button 
                        class={classes!("time-range-btn", (*selected_time_range == "1d").then(|| "active"))}
                        onclick={
                            let callback = on_time_range_change.clone();
                            Callback::from(move |_| callback.emit("1d".to_string()))
                        }
                    >
                        {"1D"}
                    </button>
                    <button 
                        class={classes!("time-range-btn", (*selected_time_range == "1w").then(|| "active"))}
                        onclick={
                            let callback = on_time_range_change.clone();
                            Callback::from(move |_| callback.emit("1w".to_string()))
                        }
                    >
                        {"1W"}
                    </button>
                    <button 
                        class={classes!("time-range-btn", (*selected_time_range == "1m").then(|| "active"))}
                        onclick={
                            let callback = on_time_range_change.clone();
                            Callback::from(move |_| callback.emit("1m".to_string()))
                        }
                    >
                        {"1M"}
                    </button>
                </div>
            </div>

            if *loading {
                <div class="pulse-viz-loading">
                    <p>{"Loading pulse data..."}</p>
                </div>
            } else if let Some(error_msg) = &*error {
                <div class="pulse-viz-error">
                    <p>{format!("Error: {}", error_msg)}</p>
                </div>
            } else {
                <div class="pulse-viz-content">
                    // Pulse History Chart
                    if let Some(history) = &*pulse_history {
                        <div class="pulse-chart-container">
                            <h4>{"Pulse Activity Chart"}</h4>
                            <div class="chart-info">
                                <p>
                                    {"Showing pulse activity from "}
                                    <strong>{format_time_short(&history.start_time)}</strong>
                                    {" to "}
                                    <strong>{format_time_short(&history.end_time)}</strong>
                                </p>
                            </div>
                            <div class="pulse-chart">
                                <PulseChart data={history.data.clone()} time_range={history.time_range.clone()} />
                            </div>
                        </div>
                    }

                    // Pulse Statistics
                    if let Some(stats) = &*pulse_stats {
                        <div class="pulse-stats-container">
                            <h4>{"Pulse Statistics"}</h4>
                            if stats.stats.is_empty() {
                                <p class="no-stats">{"No pulse data available for this time period"}</p>
                            } else {
                                <div class="stats-grid">
                                    {for stats.stats.iter().map(|stat| {
                                        html! {
                                            <div class="stat-item">
                                                <div class="stat-topic">{stat["topic"].as_str().unwrap_or("Unknown")}</div>
                                                <div class="stat-count">{stat["total_pulses"].as_i64().unwrap_or(0)}</div>
                                                <div class="stat-label">{"pulses"}</div>
                                                <div class="stat-times">
                                                    <small>
                                                        {"First: "}{format_time_short(stat["first_pulse"].as_str().unwrap_or(""))}
                                                    </small>
                                                    <small>
                                                        {"Last: "}{format_time_short(stat["last_pulse"].as_str().unwrap_or(""))}
                                                    </small>
                                                </div>
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        </div>
                    }
                </div>
            }
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct PulseChartProps {
    pub data: Vec<Value>,
    pub time_range: String,
}

#[function_component(PulseChart)]
pub fn pulse_chart(props: &PulseChartProps) -> Html {
    if props.data.is_empty() {
        return html! {
            <div class="pulse-chart-empty">
                <p>{"No pulse data available"}</p>
                <small>{"Pulses will appear here once the device starts sending pings"}</small>
            </div>
        };
    }

    // Create a simple visual representation of pulse data
    let max_pulses = props.data.iter()
        .map(|entry| entry["pulse_count"].as_i64().unwrap_or(0))
        .max()
        .unwrap_or(1);

    html! {
        <div class="pulse-chart-visualization">
            <div class="chart-legend">
                <span class="legend-item">
                    <span class="legend-color pulse-high"></span>
                    {"High Activity"}
                </span>
                <span class="legend-item">
                    <span class="legend-color pulse-medium"></span>
                    {"Medium Activity"}
                </span>
                <span class="legend-item">
                    <span class="legend-color pulse-low"></span>
                    {"Low Activity"}
                </span>
                <span class="legend-item">
                    <span class="legend-color pulse-none"></span>
                    {"No Activity"}
                </span>
            </div>
            <div class="chart-bars">
                {for props.data.iter().enumerate().map(|(index, entry)| {
                    let pulse_count = entry["pulse_count"].as_i64().unwrap_or(0);
                    let time = entry["time"].as_str().unwrap_or("");
                    let height_percent = if max_pulses > 0 {
                        (pulse_count as f64 / max_pulses as f64 * 100.0).min(100.0)
                    } else {
                        0.0
                    };
                    
                    let bar_class = if pulse_count == 0 {
                        "pulse-none"
                    } else if pulse_count < max_pulses / 3 {
                        "pulse-low"
                    } else if pulse_count < (max_pulses * 2) / 3 {
                        "pulse-medium"
                    } else {
                        "pulse-high"
                    };

                    html! {
                        <div class="chart-bar-container" title={format!("{}: {} pulses", time, pulse_count)}>
                            <div class={classes!("chart-bar", bar_class)} style={format!("height: {}%", height_percent)}>
                            </div>
                            if index % get_label_interval(&props.time_range) == 0 {
                                <div class="chart-label">{format_chart_time(time)}</div>
                            }
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

// Helper functions
async fn fetch_pulse_history(device_id: &str, time_range: &str, topic: Option<&str>) -> Result<PulseHistoryData, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let mut url = format!("/api/devices/{}/history?time_range={}", device_id, time_range);
    if let Some(topic_name) = topic {
        url.push_str(&format!("&topic={}", topic_name));
    }

    let request = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<PulseHistoryData>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}

async fn fetch_pulse_stats(device_id: &str, time_range: &str) -> Result<PulseStats, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let url = format!("/api/devices/{}/stats?time_range={}", device_id, time_range);

    let request = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<PulseStats>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}

fn format_time_short(time_str: &str) -> String {
    // Parse ISO 8601 timestamp and format to short format
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(time_str) {
        parsed.format("%m/%d %H:%M").to_string()
    } else {
        time_str.to_string()
    }
}

fn format_chart_time(time_str: &str) -> String {
    // Format time for chart labels
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(time_str) {
        parsed.format("%H:%M").to_string()
    } else {
        time_str.to_string()
    }
}

fn get_label_interval(time_range: &str) -> usize {
    // Determine how often to show labels based on time range
    match time_range {
        "1h" => 5,  // Every 5th bar for 1 hour
        "1d" => 12, // Every 12th bar for 1 day
        "1w" => 24, // Every 24th bar for 1 week
        "1m" => 60, // Every 60th bar for 1 month
        _ => 10,
    }
}
