use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, ImageData};

#[derive(Clone, PartialEq, Deserialize)]
pub struct ImageHistoryData {
    pub time_range: String,
    pub start_time: String,
    pub end_time: String,
    pub data: Vec<Value>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct ImageVisualizationProps {
    pub device_id: String,
    pub topic: String,
}

#[function_component(ImageVisualization)]
pub fn image_visualization(props: &ImageVisualizationProps) -> Html {
    let image_data = use_state(|| None::<ImageHistoryData>);
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let canvas_ref = use_node_ref();
    let selected_image_index = use_state(|| 0usize);

    // Fetch image data
    let fetch_images = {
        let device_id = props.device_id.clone();
        let topic = props.topic.clone();
        let image_data = image_data.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        Callback::from(move |_: web_sys::MouseEvent| {
            let device_id = device_id.clone();
            let topic = topic.clone();
            let image_data = image_data.clone();
            let loading = loading.clone();
            let error = error.clone();

            spawn_local(async move {
                loading.set(true);
                error.set(None);

                match fetch_image_history(&device_id, &topic).await {
                    Ok(data) => {
                        image_data.set(Some(data));
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to fetch image data: {}", e)));
                    }
                }
                loading.set(false);
            });
        })
    };

    // Initial fetch
    {
        let fetch_images = fetch_images.clone();
        use_effect_with_deps(
            move |_| {
                // Create a dummy MouseEvent for the initial fetch
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                let event = document.create_event("MouseEvent").unwrap();
                let mouse_event = event.dyn_into::<web_sys::MouseEvent>().unwrap();
                fetch_images.emit(mouse_event);
                || {}
            },
            (props.device_id.clone(), props.topic.clone()),
        );
    }

    // Render image to canvas when data changes
    {
        let canvas_ref = canvas_ref.clone();
        let image_data_for_effect = image_data.clone();
        let selected_index = *selected_image_index;
        
        use_effect_with_deps(
            move |_| {
                if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                    if let Some(data) = image_data_for_effect.as_ref() {
                        if let Some(image_entry) = data.data.get(selected_index) {
                            render_image_to_canvas(&canvas, image_entry);
                        }
                    }
                }
                || {}
            },
            ((*image_data).clone(), selected_index),
        );
    }

    let image_count = image_data.as_ref().map(|d| d.data.len()).unwrap_or(0);
    
    // Navigation callbacks
    let on_prev_image = {
        let selected_image_index = selected_image_index.clone();
        Callback::from(move |_: web_sys::MouseEvent| {
            let current = *selected_image_index;
            if current > 0 {
                selected_image_index.set(current - 1);
            }
        })
    };

    let on_next_image = {
        let selected_image_index = selected_image_index.clone();
        Callback::from(move |_: web_sys::MouseEvent| {
            let current = *selected_image_index;
            if current < image_count.saturating_sub(1) {
                selected_image_index.set(current + 1);
            }
        })
    };

    html! {
        <div class="image-visualization">
            <div class="image-viz-header">
                <h3>{"Image Data"}</h3>
                <button onclick={fetch_images} class="refresh-btn">{"Refresh"}</button>
            </div>

            if *loading {
                <div class="image-loading">{"Loading image data..."}</div>
            } else if let Some(error_msg) = (*error).as_ref() {
                <div class="image-error">{error_msg}</div>
            } else if image_count == 0 {
                <div class="image-empty">
                    <p>{"No image data available for this topic"}</p>
                    <small>{"Images will appear here when sent to this topic"}</small>
                </div>
            } else {
                <div class="image-content">
                    <div class="image-controls">
                        <button 
                            onclick={on_prev_image}
                            disabled={*selected_image_index == 0}
                            class="nav-btn"
                        >
                            {"◀ Previous"}
                        </button>
                        <span class="image-counter">
                            {format!("{} / {}", *selected_image_index + 1, image_count)}
                        </span>
                        <button 
                            onclick={on_next_image}
                            disabled={*selected_image_index >= image_count.saturating_sub(1)}
                            class="nav-btn"
                        >
                            {"Next ▶"}
                        </button>
                    </div>

                    <div class="image-display">
                        <canvas 
                            ref={canvas_ref}
                            class="image-canvas"
                        />
                        if let Some(data) = image_data.as_ref() {
                            if let Some(image_entry) = data.data.get(*selected_image_index) {
                                <div class="image-info">
                                    <div class="image-timestamp">
                                        {"Captured: "}{format_image_timestamp(image_entry)}
                                    </div>
                                    if let Some(image_obj) = image_entry.get("data").and_then(|d| d.get("Image")) {
                                        <div class="image-metadata">
                                            <span>{"Dimensions: "}{format_image_dimensions(image_obj)}</span>
                                            <span>{"Channels: "}{image_obj.get("channels").and_then(|c| c.as_u64()).unwrap_or(3)}</span>
                                            <span>{"Size: "}{format_data_size(image_obj)}</span>
                                        </div>
                                    }
                                </div>
                            }
                        }
                    </div>

                    if image_count > 1 {
                        <div class="image-thumbnails">
                            {for (0..image_count).map(|i| {
                                let is_selected = i == *selected_image_index;
                                let on_select = {
                                    let selected_image_index = selected_image_index.clone();
                                    Callback::from(move |_: web_sys::MouseEvent| {
                                        selected_image_index.set(i);
                                    })
                                };
                                
                                html! {
                                    <button 
                                        key={i}
                                        onclick={on_select}
                                        class={classes!("thumbnail-btn", is_selected.then(|| "selected"))}
                                    >
                                        {i + 1}
                                    </button>
                                }
                            })}
                        </div>
                    }
                </div>
            }
        </div>
    }
}

fn render_image_to_canvas(canvas: &HtmlCanvasElement, image_entry: &Value) {
    if let Some(image_data) = image_entry.get("data").and_then(|d| d.get("Image")) {
        let rows = image_data.get("rows").and_then(|r| r.as_u64()).unwrap_or(0) as u32;
        let cols = image_data.get("cols").and_then(|c| c.as_u64()).unwrap_or(0) as u32;
        let channels = image_data.get("channels").and_then(|c| c.as_u64()).unwrap_or(3) as u32;
        let data = image_data.get("data").and_then(|d| d.as_array());

        if let Some(pixel_data) = data {
            // Set canvas dimensions
            canvas.set_width(cols);
            canvas.set_height(rows);

            if let Ok(context) = canvas.get_context("2d").unwrap().unwrap().dyn_into::<CanvasRenderingContext2d>() {
                // Convert pixel data to RGBA format for canvas
                let mut rgba_data = Vec::with_capacity((rows * cols * 4) as usize);
                
                for i in 0..(rows * cols) {
                    let base_idx = (i * channels) as usize;
                    
                    if channels >= 3 && base_idx + 2 < pixel_data.len() {
                        // RGB or RGBA
                        let r = pixel_data[base_idx].as_u64().unwrap_or(0) as u8;
                        let g = pixel_data[base_idx + 1].as_u64().unwrap_or(0) as u8;
                        let b = pixel_data[base_idx + 2].as_u64().unwrap_or(0) as u8;
                        let a = if channels >= 4 && base_idx + 3 < pixel_data.len() {
                            pixel_data[base_idx + 3].as_u64().unwrap_or(255) as u8
                        } else {
                            255u8
                        };
                        
                        rgba_data.push(r);
                        rgba_data.push(g);
                        rgba_data.push(b);
                        rgba_data.push(a);
                    } else if channels == 1 && base_idx < pixel_data.len() {
                        // Grayscale
                        let gray = pixel_data[base_idx].as_u64().unwrap_or(0) as u8;
                        rgba_data.push(gray);
                        rgba_data.push(gray);
                        rgba_data.push(gray);
                        rgba_data.push(255u8);
                    } else {
                        // Default to black pixel
                        rgba_data.push(0);
                        rgba_data.push(0);
                        rgba_data.push(0);
                        rgba_data.push(255u8);
                    }
                }

                // Create ImageData and put it on canvas
                if let Ok(img_data) = ImageData::new_with_u8_clamped_array_and_sh(
                    wasm_bindgen::Clamped(&rgba_data[..]), cols, rows
                ) {
                    let _ = context.put_image_data(&img_data, 0.0, 0.0);
                }
            }
        }
    }
}

fn format_image_timestamp(image_entry: &Value) -> String {
    if let Some(timestamp) = image_entry.get("timestamp").and_then(|t| t.as_str()) {
        // Parse timestamp and format it nicely in European format
        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(timestamp) {
            datetime.format("%d/%m/%Y %H:%M:%S").to_string()
        } else {
            timestamp.to_string()
        }
    } else {
        "Unknown".to_string()
    }
}

fn format_image_dimensions(image_obj: &Value) -> String {
    let rows = image_obj.get("rows").and_then(|r| r.as_u64()).unwrap_or(0);
    let cols = image_obj.get("cols").and_then(|c| c.as_u64()).unwrap_or(0);
    format!("{}×{}", cols, rows)
}

fn format_data_size(image_obj: &Value) -> String {
    if let Some(data) = image_obj.get("data").and_then(|d| d.as_array()) {
        let size_bytes = data.len();
        if size_bytes >= 1024 * 1024 {
            format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
        } else if size_bytes >= 1024 {
            format!("{:.1} KB", size_bytes as f64 / 1024.0)
        } else {
            format!("{} bytes", size_bytes)
        }
    } else {
        "Unknown".to_string()
    }
}

async fn fetch_image_history(device_id: &str, topic: &str) -> Result<ImageHistoryData, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    let url = format!("/api/devices/{}/data?topic={}&type=image", device_id, topic);

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

        Ok(ImageHistoryData {
            time_range: "1d".to_string(),
            start_time: "".to_string(),
            end_time: "".to_string(),
            data,
        })
    } else {
        Err(format!("Server error: {}", request.status()))
    }
}
