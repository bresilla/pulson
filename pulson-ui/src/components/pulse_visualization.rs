use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use serde::{Deserialize};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use chrono::{DateTime, Utc, Duration, Timelike};

#[derive(Clone, PartialEq, Deserialize)]
pub struct PulseHistoryData {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub data: Vec<Value>,
}

#[derive(Clone, PartialEq)]
pub enum DominoStatus {
    Green,   // Recent pulse
    Orange,  // Some activity but not recent
    Red,     // No activity
    Gray,    // No data available
}

#[derive(Clone, PartialEq)]
pub struct DominoBox {
    pub status: DominoStatus,
    pub tooltip: String,
    pub label: String,
}

#[derive(Clone, PartialEq)]
pub struct PulseStatistics {
    pub total_pulses: i64,
    pub active_intervals: usize,
    pub avg_time_between_pulses: Option<f64>, // in minutes
    pub most_recent_pulse: Option<String>,
    pub peak_activity_interval: Option<String>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct PulseVisualizationProps {
    pub device_id: String,
    pub topic: Option<String>,
}

#[function_component(PulseVisualization)]
pub fn pulse_visualization(props: &PulseVisualizationProps) -> Html {
    let pulse_history = use_state(|| None::<PulseHistoryData>);
    let selected_time_range = use_state(|| "1h".to_string());
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let refresh_interval = use_state(|| None::<Interval>);

    // Auto-refresh function
    let refresh_data = {
        let device_id = props.device_id.clone();
        let topic = props.topic.clone();
        let pulse_history = pulse_history.clone();
        let loading = loading.clone();
        let error = error.clone();
        let selected_time_range = selected_time_range.clone();
        
        Callback::from(move |_| {
            let device_id = device_id.clone();
            let topic = topic.clone();
            let time_range = (*selected_time_range).clone();
            let pulse_history = pulse_history.clone();
            let _loading = loading.clone();
            let error = error.clone();

            spawn_local(async move {
                // Don't show loading for auto-refresh
                error.set(None);

                match fetch_pulse_history(&device_id, &time_range, topic.as_deref()).await {
                    Ok(history) => {
                        pulse_history.set(Some(history));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch pulse history: {}", e)));
                    }
                }
            });
        })
    };

    // Set up auto-refresh interval
    {
        let refresh_data = refresh_data.clone();
        let refresh_interval = refresh_interval.clone();
        
        use_effect_with_deps(
            move |_| {
                // Clear existing interval
                refresh_interval.set(None);
                
                // Set up new interval (refresh every 5 seconds)
                let interval = Interval::new(5000, move || {
                    refresh_data.emit(());
                });
                
                refresh_interval.set(Some(interval));
                
                || {
                    // Cleanup on unmount
                }
            },
            (),
        );
    }

    // Initial fetch when device, topic, or time range changes
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
            (props.device_id.clone(), props.topic.clone(), (*selected_time_range).clone()),
        );
    }

    // Time range selection callback
    let on_time_range_change = {
        let selected_time_range = selected_time_range.clone();
        Callback::from(move |time_range: String| {
            selected_time_range.set(time_range);
        })
    };

    // Generate domino boxes based on time range and pulse data
    let domino_boxes = generate_domino_boxes(&selected_time_range, &pulse_history);
    
    // Calculate pulse statistics
    let statistics = calculate_pulse_statistics(&selected_time_range, &pulse_history);

    html! {
        <div class="pulse-visualization">
            <div class="pulse-viz-header">
                <h3>{"Pulse Activity"}</h3>
                <div class="time-range-selector">
                    {for ["1h", "1d", "1w", "1m"].iter().map(|&range| {
                        let range_str = range.to_string();
                        let is_selected = *selected_time_range == range_str;
                        let on_click = {
                            let range_str = range_str.clone();
                            let on_time_range_change = on_time_range_change.clone();
                            Callback::from(move |_| {
                                on_time_range_change.emit(range_str.clone());
                            })
                        };
                        html! {
                            <button 
                                class={classes!("btn", "btn-small", is_selected.then(|| "btn-active"))}
                                onclick={on_click}
                            >
                                {range.to_uppercase()}
                            </button>
                        }
                    })}
                </div>
            </div>

            if *loading {
                <div class="pulse-loading">{"Loading pulse data..."}</div>
            } else if let Some(error_msg) = (*error).as_ref() {
                <div class="pulse-error">{error_msg}</div>
            } else {
                <div class="domino-container">
                    <div class="domino-info">
                        <p>{get_time_range_description(&selected_time_range)}</p>
                    </div>
                    <div class="pulse-statistics">
                        <div class="stat-item">
                            <span class="stat-label">{"Total pulses:"}</span>
                            <span class="stat-value">{statistics.total_pulses}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Active intervals:"}</span>
                            <span class="stat-value">{format!("{}/{}", statistics.active_intervals, domino_boxes.len())}</span>
                        </div>
                        if let Some(avg_time) = statistics.avg_time_between_pulses {
                            <div class="stat-item">
                                <span class="stat-label">{"Avg time between:"}</span>
                                <span class="stat-value">{format!("{:.1} min", avg_time)}</span>
                            </div>
                        }
                        if let Some(ref recent_pulse) = statistics.most_recent_pulse {
                            <div class="stat-item">
                                <span class="stat-label">{"Last pulse:"}</span>
                                <span class="stat-value">{recent_pulse}</span>
                            </div>
                        }
                    </div>
                    <div class="domino-grid">
                        {for domino_boxes.into_iter().enumerate().map(|(index, domino_box)| {
                            let status_class = match domino_box.status {
                                DominoStatus::Green => "domino-green",
                                DominoStatus::Orange => "domino-orange", 
                                DominoStatus::Red => "domino-red",
                                DominoStatus::Gray => "domino-gray",
                            };
                            html! {
                                <div 
                                    class={classes!("domino-box", status_class)}
                                    title={domino_box.tooltip.clone()}
                                    key={index}
                                >
                                    <div class="domino-inner"></div>
                                </div>
                            }
                        })}
                    </div>
                    <div class="domino-legend">
                        <div class="legend-item">
                            <div class="legend-color domino-green"></div>
                            <span>{"Recent activity"}</span>
                        </div>
                        <div class="legend-item">
                            <div class="legend-color domino-orange"></div>
                            <span>{"Some activity"}</span>
                        </div>
                        <div class="legend-item">
                            <div class="legend-color domino-red"></div>
                            <span>{"No activity"}</span>
                        </div>
                        <div class="legend-item">
                            <div class="legend-color domino-gray"></div>
                            <span>{"No data"}</span>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}

// Generate domino boxes based on time range and pulse data
fn generate_domino_boxes(time_range: &str, pulse_history: &UseStateHandle<Option<PulseHistoryData>>) -> Vec<DominoBox> {
    let box_count = match time_range {
        "1h" => 30,
        "1d" => 24,
        "1w" => 14,
        "1m" => 30,
        _ => 30,
    };

    // If no pulse data available, return all gray boxes
    let pulse_data = match pulse_history.as_ref() {
        Some(data) => &data.data,
        None => {
            return (0..box_count).map(|i| DominoBox {
                status: DominoStatus::Gray,
                tooltip: format!("Interval {}: No data available", i + 1),
                label: String::new(), // No labels for 100 tiny boxes
            }).collect();
        }
    };

    // Generate boxes based on the selected time range
    let now = Utc::now();
    let mut boxes = Vec::with_capacity(box_count);

    for i in 0..box_count {
        let (start_time, end_time, label) = calculate_interval_times(time_range, i, box_count, &now);
        
        // Check if there were any pulses in this interval
        let pulse_count = count_pulses_in_interval(pulse_data, &start_time, &end_time);
        
        // Determine the color based on pulse activity
        let status = if pulse_count > 0 {
            // Check if this is the most recent interval with activity
            if i == box_count - 1 || is_most_recent_with_activity(pulse_data, &start_time, &end_time, time_range, i, box_count, &now) {
                DominoStatus::Green
            } else {
                DominoStatus::Orange
            }
        } else {
            DominoStatus::Red
        };

        boxes.push(DominoBox {
            status,
            tooltip: format!("Interval {} ({}): {} pulse{}", i + 1, label, pulse_count, if pulse_count == 1 { "" } else { "s" }),
            label: String::new(), // No labels for 100 tiny boxes
        });
    }

    boxes
}

// Calculate pulse statistics based on the pulse data
fn calculate_pulse_statistics(time_range: &str, pulse_history: &UseStateHandle<Option<PulseHistoryData>>) -> PulseStatistics {
    let pulse_data = match pulse_history.as_ref() {
        Some(data) => &data.data,
        None => {
            return PulseStatistics {
                total_pulses: 0,
                active_intervals: 0,
                avg_time_between_pulses: None,
                most_recent_pulse: None,
                peak_activity_interval: None,
            };
        }
    };

    // Parse all pulse timestamps and counts
    let mut pulse_events: Vec<(DateTime<Utc>, i64)> = Vec::new();
    let mut total_pulses = 0i64;
    
    for entry in pulse_data {
        // Try both "timestamp" and "time" field names for compatibility
        if let Some(timestamp_str) = entry["timestamp"].as_str().or_else(|| entry["time"].as_str()) {
            if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp_str) {
                let timestamp = timestamp.with_timezone(&Utc);
                let pulse_count = entry["pulse_count"].as_i64().unwrap_or(1);
                total_pulses += pulse_count;
                pulse_events.push((timestamp, pulse_count));
            }
        }
    }
    
    // Sort by timestamp
    pulse_events.sort_by_key(|(timestamp, _)| *timestamp);
    
    // Calculate active intervals by checking how many time intervals had activity
    let now = Utc::now();
    let box_count = match time_range {
        "1h" => 30,
        "1d" => 24,
        "1w" => 14,
        "1m" => 30,
        _ => 30,
    };
    
    let mut active_intervals = 0;
    for i in 0..box_count {
        let (start_time, end_time, _) = calculate_interval_times(time_range, i, box_count, &now);
        let pulse_count = count_pulses_in_interval(pulse_data, &start_time, &end_time);
        if pulse_count > 0 {
            active_intervals += 1;
        }
    }
    
    // Calculate average time between pulses
    let avg_time_between_pulses = if pulse_events.len() > 1 {
        let total_duration = pulse_events.last().unwrap().0.signed_duration_since(pulse_events.first().unwrap().0);
        Some(total_duration.num_minutes() as f64 / (pulse_events.len() - 1) as f64)
    } else {
        None
    };
    
    // Find most recent pulse
    let most_recent_pulse = pulse_events.last().map(|(timestamp, _)| {
        let duration_ago = now.signed_duration_since(*timestamp);
        if duration_ago.num_minutes() < 60 {
            format!("{}m ago", duration_ago.num_minutes())
        } else if duration_ago.num_hours() < 24 {
            format!("{}h ago", duration_ago.num_hours())
        } else {
            format!("{}d ago", duration_ago.num_days())
        }
    });
    
    PulseStatistics {
        total_pulses,
        active_intervals,
        avg_time_between_pulses,
        most_recent_pulse,
        peak_activity_interval: None, // We can implement this later if needed
    }
}

fn calculate_interval_times(time_range: &str, index: usize, total_boxes: usize, now: &DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>, String) {
    match time_range {
        "1h" => {
            // 30 boxes, 2-minute intervals
            let minutes_back = (total_boxes - index) * 2;
            let end_time = *now - Duration::minutes((minutes_back - 2) as i64);
            let start_time = end_time - Duration::minutes(2);
            let label = format!("{}:{:02}", start_time.hour(), start_time.minute());
            (start_time, end_time, label)
        },
        "1d" => {
            // 24 boxes, hourly intervals
            let hours_back = total_boxes - index;
            let end_time = *now - Duration::hours((hours_back - 1) as i64);
            let start_time = end_time - Duration::hours(1);
            let label = format!("{}:00", start_time.hour());
            (start_time, end_time, label)
        },
        "1w" => {
            // 14 boxes, 12-hour intervals (day/night)
            let intervals_back = total_boxes - index;
            let hours_back = intervals_back * 12;
            let end_time = *now - Duration::hours((hours_back - 12) as i64);
            let start_time = end_time - Duration::hours(12);
            let period = if start_time.hour() < 12 { "AM" } else { "PM" };
            let label = format!("{} {}", start_time.format("%d/%m"), period);
            (start_time, end_time, label)
        },
        "1m" => {
            // 30 boxes, daily intervals
            let days_back = total_boxes - index;
            let end_time = *now - Duration::days((days_back - 1) as i64);
            let start_time = end_time - Duration::days(1);
            let label = start_time.format("%d/%m").to_string();
            (start_time, end_time, label)
        },
        _ => {
            let end_time = *now;
            let start_time = end_time - Duration::hours(1);
            (start_time, end_time, "Unknown".to_string())
        }
    }
}

fn count_pulses_in_interval(pulse_data: &[Value], start_time: &DateTime<Utc>, end_time: &DateTime<Utc>) -> i64 {
    pulse_data.iter()
        .filter_map(|entry| {
            let timestamp_str = entry["timestamp"].as_str()?;
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str).ok()?.with_timezone(&Utc);
            if timestamp >= *start_time && timestamp < *end_time {
                Some(entry["pulse_count"].as_i64().unwrap_or(1))
            } else {
                None
            }
        })
        .sum()
}

fn is_most_recent_with_activity(
    pulse_data: &[Value], 
    current_start: &DateTime<Utc>, 
    current_end: &DateTime<Utc>,
    _time_range: &str,
    _current_index: usize,
    _total_boxes: usize,
    _now: &DateTime<Utc>
) -> bool {
    // Check if this interval contains the most recent pulse activity
    let mut most_recent_pulse: Option<DateTime<Utc>> = None;
    
    // Find the most recent pulse across all intervals
    for entry in pulse_data {
        // Try both "timestamp" and "time" field names for compatibility
        if let Some(timestamp_str) = entry["timestamp"].as_str().or_else(|| entry["time"].as_str()) {
            if let Ok(timestamp) = DateTime::parse_from_rfc3339(timestamp_str) {
                let timestamp = timestamp.with_timezone(&Utc);
                if entry["pulse_count"].as_i64().unwrap_or(0) > 0 {
                    most_recent_pulse = Some(most_recent_pulse.map_or(timestamp, |prev| prev.max(timestamp)));
                }
            }
        }
    }
    
    // Check if the most recent pulse falls within this interval
    if let Some(recent_pulse) = most_recent_pulse {
        recent_pulse >= *current_start && recent_pulse < *current_end
    } else {
        false
    }
}

fn get_time_range_description(time_range: &str) -> String {
    match time_range {
        "1h" => "30 boxes, 2-minute intervals".to_string(),
        "1d" => "24 boxes, hourly intervals".to_string(),
        "1w" => "14 boxes, 12-hour day/night intervals".to_string(),
        "1m" => "30 boxes, daily intervals".to_string(),
        _ => "Time intervals".to_string(),
    }
}

// Helper functions for API calls
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
