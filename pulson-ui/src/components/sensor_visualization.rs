use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Interval;
use serde::{Deserialize};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Clone, PartialEq, Deserialize)]
pub struct SensorHistoryData {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub data: Vec<Value>,
}

#[derive(Clone, PartialEq)]
pub struct SensorReading {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct SensorVisualizationProps {
    pub device_id: String,
    pub topic: String,
}

#[function_component(SensorVisualization)]
pub fn sensor_visualization(props: &SensorVisualizationProps) -> Html {
    let sensor_history = use_state(|| None::<SensorHistoryData>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let refresh_interval = use_state(|| None::<Interval>);

    // Auto-refresh function
    let refresh_data = {
        let device_id = props.device_id.clone();
        let topic = props.topic.clone();
        let sensor_history = sensor_history.clone();
        let error = error.clone();
        
        Callback::from(move |_| {
            let device_id = device_id.clone();
            let topic = topic.clone();
            let sensor_history = sensor_history.clone();
            let error = error.clone();

            spawn_local(async move {
                error.set(None);

                match fetch_sensor_history(&device_id, &topic).await {
                    Ok(history) => {
                        sensor_history.set(Some(history));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch sensor data: {}", e)));
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

    // Parse latest sensor reading
    let latest_reading = sensor_history.as_ref().and_then(|history| {
        parse_latest_sensor_reading(&history.data)
    });

    // Parse historical readings for trend chart
    let historical_readings = sensor_history.as_ref()
        .map(|history| parse_sensor_readings(&history.data))
        .unwrap_or_default();

    html! {
        <div class="sensor-visualization">
            <div class="sensor-viz-header">
                <h3>{"Sensor Data: "}{&props.topic}</h3>
                <button onclick={manual_refresh} class="refresh-btn">{"Refresh"}</button>
            </div>

            if *loading {
                <div class="sensor-loading">{"Loading sensor data..."}</div>
            } else if let Some(error_msg) = (*error).as_ref() {
                <div class="sensor-error">{error_msg}</div>
            } else if let Some(reading) = latest_reading {
                <div class="sensor-content">
                    // Main Gauge Display
                    <div class="sensor-gauge-container">
                        <div class="sensor-gauge">
                            <svg viewBox="0 0 200 120" class="gauge-svg">
                                // Background arc
                                <path
                                    d="M 20 100 A 80 80 0 0 1 180 100"
                                    fill="none"
                                    stroke="#3a3a40"
                                    stroke-width="12"
                                    stroke-linecap="round"
                                />
                                // Value arc
                                <path
                                    d="M 20 100 A 80 80 0 0 1 180 100"
                                    fill="none"
                                    stroke={get_gauge_color(reading.value, reading.min, reading.max)}
                                    stroke-width="12"
                                    stroke-linecap="round"
                                    stroke-dasharray={get_gauge_dasharray(reading.value, reading.min, reading.max)}
                                    class="gauge-arc"
                                />
                                // Center text
                                <text x="100" y="85" text-anchor="middle" class="gauge-value">
                                    {format!("{:.1}", reading.value)}
                                </text>
                                <text x="100" y="105" text-anchor="middle" class="gauge-unit">
                                    {"sensor"}
                                </text>
                            </svg>
                        </div>
                        <div class="sensor-info">
                            <div class="sensor-range">
                                <span class="range-label">{"Range:"}</span>
                                <span class="range-value">{format!("{:.1} - {:.1}", reading.min, reading.max)}</span>
                            </div>
                            <div class="sensor-percentage">
                                <span class="percentage-label">{"Percentage:"}</span>
                                <span class="percentage-value">
                                    {format!("{:.1}%", calculate_percentage(reading.value, reading.min, reading.max))}
                                </span>
                            </div>
                        </div>
                    </div>

                    // Historical Trend Chart
                    if !historical_readings.is_empty() {
                        <div class="sensor-trend">
                            <h4>{"Recent Trend"}</h4>
                            <div class="trend-chart">
                                <svg viewBox="0 0 400 100" class="trend-svg">
                                    // Background grid
                                    {for (1..5).map(|i| {
                                        let y = i as f64 * 20.0;
                                        html! {
                                            <line
                                                x1="0"
                                                y1={y.to_string()}
                                                x2="400"
                                                y2={y.to_string()}
                                                stroke="#2a2a30"
                                                stroke-width="1"
                                            />
                                        }
                                    })}
                                    
                                    // Trend line
                                    <polyline
                                        points={generate_trend_points(&historical_readings)}
                                        fill="none"
                                        stroke="#eb1c24"
                                        stroke-width="2"
                                        class="trend-line"
                                    />
                                    
                                    // Data points
                                    {for historical_readings.iter().enumerate().map(|(i, reading)| {
                                        let x = (i as f64 / (historical_readings.len() - 1).max(1) as f64) * 380.0 + 10.0;
                                        let y = 90.0 - ((reading.value - reading.min) / (reading.max - reading.min)) * 80.0;
                                        html! {
                                            <circle
                                                cx={x.to_string()}
                                                cy={y.to_string()}
                                                r="3"
                                                fill="#eb1c24"
                                                class="trend-point"
                                            >
                                                <title>{format!("Value: {:.1} at {}", reading.value, reading.timestamp.format("%H:%M"))}</title>
                                            </circle>
                                        }
                                    })}
                                </svg>
                            </div>
                            <div class="trend-stats">
                                <div class="stat-item">
                                    <span class="stat-label">{"Latest:"}</span>
                                    <span class="stat-value">{format!("{:.1}", reading.value)}</span>
                                </div>
                                <div class="stat-item">
                                    <span class="stat-label">{"Average:"}</span>
                                    <span class="stat-value">
                                        {format!("{:.1}", calculate_average(&historical_readings))}
                                    </span>
                                </div>
                                <div class="stat-item">
                                    <span class="stat-label">{"Readings:"}</span>
                                    <span class="stat-value">{historical_readings.len()}</span>
                                </div>
                            </div>
                        </div>
                    }
                </div>
            } else {
                <div class="sensor-empty">
                    <p>{"No sensor data available for this topic"}</p>
                    <small>{"Sensor readings will appear here when data is received"}</small>
                </div>
            }
        </div>
    }
}

// Helper functions

fn parse_latest_sensor_reading(data: &[Value]) -> Option<SensorReading> {
    data.last().and_then(|entry| {
        let timestamp_str = entry["timestamp"].as_str()?;
        let timestamp = DateTime::parse_from_rfc3339(timestamp_str).ok()?.with_timezone(&Utc);
        
        if let Some(sensor_data) = entry["data"].as_object() {
            if let Some(sensor_obj) = sensor_data.get("Sensor").and_then(|s| s.as_object()) {
                let value = sensor_obj.get("value").and_then(|v| v.as_f64())?;
                let min = sensor_obj.get("min").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let max = sensor_obj.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
                
                return Some(SensorReading {
                    value,
                    min,
                    max,
                    timestamp,
                });
            }
        }
        
        None
    })
}

fn parse_sensor_readings(data: &[Value]) -> Vec<SensorReading> {
    data.iter()
        .filter_map(|entry| {
            let timestamp_str = entry["timestamp"].as_str()?;
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str).ok()?.with_timezone(&Utc);
            
            if let Some(sensor_data) = entry["data"].as_object() {
                if let Some(sensor_obj) = sensor_data.get("Sensor").and_then(|s| s.as_object()) {
                    let value = sensor_obj.get("value").and_then(|v| v.as_f64())?;
                    let min = sensor_obj.get("min").and_then(|v| v.as_f64()).unwrap_or(1.0);
                    let max = sensor_obj.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
                    
                    return Some(SensorReading {
                        value,
                        min,
                        max,
                        timestamp,
                    });
                }
            }
            
            None
        })
        .collect()
}

fn get_gauge_color(value: f64, min: f64, max: f64) -> &'static str {
    let percentage = calculate_percentage(value, min, max);
    
    if percentage >= 80.0 {
        "#e74c3c" // Red for high values
    } else if percentage >= 60.0 {
        "#f39c12" // Orange for medium-high values
    } else if percentage >= 40.0 {
        "#f1c40f" // Yellow for medium values
    } else if percentage >= 20.0 {
        "#2ecc71" // Green for good values
    } else {
        "#3498db" // Blue for low values
    }
}

fn get_gauge_dasharray(value: f64, min: f64, max: f64) -> String {
    let percentage = calculate_percentage(value, min, max);
    let arc_length = 251.3; // Approximate arc length for 180 degrees
    let fill_length = (percentage / 100.0) * arc_length;
    let empty_length = arc_length - fill_length;
    
    format!("{:.1} {:.1}", fill_length, empty_length)
}

fn calculate_percentage(value: f64, min: f64, max: f64) -> f64 {
    if max == min {
        return 50.0; // Default to 50% if range is invalid
    }
    
    ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
}

fn generate_trend_points(readings: &[SensorReading]) -> String {
    if readings.is_empty() {
        return String::new();
    }
    
    let points: Vec<String> = readings
        .iter()
        .enumerate()
        .map(|(i, reading)| {
            let x = (i as f64 / (readings.len() - 1).max(1) as f64) * 380.0 + 10.0;
            let y = 90.0 - ((reading.value - reading.min) / (reading.max - reading.min)) * 80.0;
            format!("{:.1},{:.1}", x, y)
        })
        .collect();
    
    points.join(" ")
}

fn calculate_average(readings: &[SensorReading]) -> f64 {
    if readings.is_empty() {
        return 0.0;
    }
    
    let sum: f64 = readings.iter().map(|r| r.value).sum();
    sum / readings.len() as f64
}

// API call to fetch sensor history
async fn fetch_sensor_history(device_id: &str, topic: &str) -> Result<SensorHistoryData, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let url = format!("/api/devices/{}/history?time_range=1h&topic={}", device_id, topic);

    let request = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() == 200 {
        request
            .json::<SensorHistoryData>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}
