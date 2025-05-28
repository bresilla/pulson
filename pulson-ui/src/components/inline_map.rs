use gloo_net::http::Request;
use gloo_storage::{LocalStorage, Storage};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use web_sys::Element;
use yew::prelude::*;
use yew::{use_effect_with_deps, use_state};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, PartialEq, Deserialize)]
pub struct DeviceLocation {
    pub device_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub topic: String,
    pub timestamp: String,
    pub status: String,
}

#[derive(Properties, Clone, PartialEq)]
pub struct InlineMapProps {
    pub device_id: String,
    pub topic: String,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = L)]
    type Map;

    #[wasm_bindgen(js_namespace = L)]
    type TileLayer;

    #[wasm_bindgen(js_namespace = L)]
    type CircleMarker;

    #[wasm_bindgen(js_namespace = L)]
    type LatLng;

    #[wasm_bindgen(constructor, js_namespace = L)]
    fn new(element: &Element, options: &JsValue) -> Map;

    #[wasm_bindgen(constructor, js_namespace = L)]
    fn new(url: &str, options: &JsValue) -> TileLayer;

    #[wasm_bindgen(constructor, js_namespace = L)]
    fn new(latlng: &LatLng, options: &JsValue) -> CircleMarker;

    #[wasm_bindgen(constructor, js_namespace = L)]
    fn new(lat: f64, lng: f64) -> LatLng;

    #[wasm_bindgen(method, js_name = addTo)]
    fn add_to(this: &TileLayer, map: &Map);

    #[wasm_bindgen(method, js_name = setView)]
    fn set_view(this: &Map, center: &LatLng, zoom: u8);

    #[wasm_bindgen(method, js_name = addTo)]
    fn add_marker_to(this: &CircleMarker, map: &Map);

    #[wasm_bindgen(method, js_name = bindPopup)]
    fn bind_popup(this: &CircleMarker, content: &str);

    #[wasm_bindgen(method, js_name = invalidateSize)]
    fn invalidate_size(this: &Map, options: &JsValue);
}

#[function_component(InlineMap)]
pub fn inline_map_component(props: &InlineMapProps) -> Html {
    let map_ref = use_node_ref();
    let locations = use_state(Vec::<DeviceLocation>::new);
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    // Fetch location data for this specific device/topic
    {
        let locations = locations.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with_deps(
            move |deps: &(String, String)| {
                let (device_id, topic) = deps.clone();
                let locations_setter = locations.clone();
                let loading_setter = loading.clone();
                let error_setter = error.clone();

                // Use a more isolated async operation to prevent parent re-renders
                spawn_local(async move {
                    loading_setter.set(true);
                    error_setter.set(None);

                    match fetch_device_location(&device_id, &topic).await {
                        Ok(Some(location)) => {
                            locations_setter.set(vec![location]);
                        }
                        Ok(None) => {
                            error_setter.set(Some("No location data found for this topic".to_string()));
                        }
                        Err(e) => {
                            error_setter.set(Some(e));
                        }
                    }

                    loading_setter.set(false);
                });

                || {}
            },
            (props.device_id.clone(), props.topic.clone()),
        );
    }

    // Initialize map when locations are loaded
    {
        let map_ref = map_ref.clone();
        
        use_effect_with_deps(
            move |locations_deps: &Vec<DeviceLocation>| {
                if !locations_deps.is_empty() {
                    if let Some(map_element) = map_ref.cast::<Element>() {
                        // Add a small delay to ensure DOM is fully rendered
                        let element_clone = map_element.clone();
                        let locations_clone = locations_deps.clone();
                        
                        let timeout = gloo_timers::callback::Timeout::new(250, move || {
                            initialize_inline_map(&element_clone, &locations_clone);
                        });
                        timeout.forget(); // Let it run
                    }
                }
                || {}
            },
            (*locations).clone(),
        );
    }

    html! {
        <div class="inline-map-container">
            if *loading {
                <div class="inline-map-loading">
                    <div class="loading-spinner-small"></div>
                    <span>{"Loading location..."}</span>
                </div>
            } else if let Some(error_msg) = &*error {
                <div class="inline-map-error">
                    <span>{"❌ "}{error_msg}</span>
                </div>
            } else if locations.is_empty() {
                <div class="inline-map-empty">
                    <span>{"📍 No location data available"}</span>
                </div>
            } else {
                <div class="inline-map" ref={map_ref}></div>
            }
        </div>
    }
}

fn initialize_inline_map(element: &Element, locations: &[DeviceLocation]) {
    // Check if Leaflet is available
    if let Some(window) = web_sys::window() {
        if let Ok(l_obj) = js_sys::Reflect::get(&window, &"L".into()) {
            if l_obj.is_undefined() {
                return;
            }
        }
    }
    
    // Clear any existing map
    element.set_inner_html("");
    
    // Check element dimensions and ensure proper size
    let rect = element.get_bounding_client_rect();
        
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        // Force dimensions by setting style directly
        if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
            let _ = html_element.style().set_property("width", "100%");
            let _ = html_element.style().set_property("height", "500px");
            let _ = html_element.style().set_property("min-height", "500px");
            let _ = html_element.style().set_property("display", "block");
        }
    }
    
    // Create map options
    let map_options = js_sys::Object::new();
    js_sys::Reflect::set(&map_options, &"zoom".into(), &13.into()).unwrap();
    js_sys::Reflect::set(&map_options, &"zoomControl".into(), &false.into()).unwrap();
    js_sys::Reflect::set(&map_options, &"attributionControl".into(), &false.into()).unwrap();
    
    // Initialize map
    let map = Map::new(element, &map_options.into());
    
    // Force map to recalculate its size
    let invalidate_options = js_sys::Object::new();
    map.invalidate_size(&invalidate_options.into());
    
    // Add CSS class to map container for event isolation
    let class_list = element.class_list();
    let _ = class_list.add_1("leaflet-static-map");
    
    // Add tile layer (OpenStreetMap)
    let tile_options = js_sys::Object::new();
    js_sys::Reflect::set(
        &tile_options,
        &"attribution".into(),
        &"© OpenStreetMap".into(),
    ).unwrap();
    
    let tile_layer = TileLayer::new(
        "https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png",
        &tile_options.into(),
    );
    
    tile_layer.add_to(&map);
    
    if let Some(location) = locations.first() {
        let center = LatLng::new(location.latitude, location.longitude);
        
        // Set initial view
        map.set_view(&center, 13);
        
        // Force another size invalidation after view is set
        let invalidate_options2 = js_sys::Object::new();
        map.invalidate_size(&invalidate_options2.into());
        
        // Add circle marker with status color
        add_status_marker(&map, location);
    }
}

fn add_status_marker(map: &Map, location: &DeviceLocation) {
    let latlng = LatLng::new(location.latitude, location.longitude);
    
    // Create circle marker options with status-based colors
    let marker_options = js_sys::Object::new();
    
    let (color, fill_color) = match location.status.as_str() {
        "Online" => ("#22c55e", "#22c55e"),     // Green
        "Warning" => ("#f59e0b", "#f59e0b"),    // Amber
        _ => ("#ef4444", "#ef4444"),             // Red
    };
    
    js_sys::Reflect::set(&marker_options, &"color".into(), &color.into()).unwrap();
    js_sys::Reflect::set(&marker_options, &"fillColor".into(), &fill_color.into()).unwrap();
    js_sys::Reflect::set(&marker_options, &"fillOpacity".into(), &0.8.into()).unwrap();
    js_sys::Reflect::set(&marker_options, &"radius".into(), &8.into()).unwrap();
    js_sys::Reflect::set(&marker_options, &"weight".into(), &2.into()).unwrap();
    
    let marker = CircleMarker::new(&latlng, &marker_options.into());
    
    // Create popup content
    let popup_content = format!(
        r#"
        <div class="marker-popup">
            <h4>{}</h4>
            <p><strong>Topic:</strong> {}</p>
            <p><strong>Status:</strong> <span class="status {}">{}</span></p>
            <p><strong>Coordinates:</strong> {:.6}, {:.6}</p>
            <p><strong>Last Update:</strong> {}</p>
        </div>
        "#,
        location.device_id,
        location.topic,
        location.status.to_lowercase(),
        location.status,
        location.latitude,
        location.longitude,
        location.timestamp
    );
    
    marker.bind_popup(&popup_content);
    marker.add_marker_to(map);
}

async fn fetch_device_location(device_id: &str, topic: &str) -> Result<Option<DeviceLocation>, String> {
    let token = LocalStorage::get::<String>("pulson_token")
        .map_err(|_| "No authentication token found".to_string())?;

    // Get device info for status
    let device_request = Request::get("/api/devices")
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error fetching devices: {}", e))?;

    if device_request.status() != 200 {
        return Err(format!("Failed to fetch devices: {}", device_request.status()));
    }

    let devices: Vec<serde_json::Value> = device_request
        .json()
        .await
        .map_err(|e| format!("Failed to parse devices response: {}", e))?;

    let device_status = devices
        .iter()
        .find(|d| d["device_id"].as_str() == Some(device_id))
        .and_then(|d| d["status"].as_str())
        .unwrap_or("Unknown")
        .to_string();

    // Get latest data for this specific topic
    match fetch_latest_location_data(device_id, topic, &token).await {
        Ok(Some((lat, lng, timestamp))) => {
            Ok(Some(DeviceLocation {
                device_id: device_id.to_string(),
                latitude: lat,
                longitude: lng,
                topic: topic.to_string(),
                timestamp,
                status: device_status,
            }))
        }
        Ok(None) => {
            Ok(None)
        }
        Err(e) => {
            Err(e)
        }
    }
}

async fn fetch_latest_location_data(device_id: &str, topic: &str, token: &str) -> Result<Option<(f64, f64, String)>, String> {
    let url = format!("/api/devices/{}/data?topic={}", device_id, topic);
    
    let request = Request::get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if request.status() != 200 {
        return Ok(None);
    }

    let response: serde_json::Value = request
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Try to extract coordinates from the latest data entry
    if let Some(data_array) = response["data"].as_array() {
        if let Some(latest_entry) = data_array.first() {
            
            // Try to get data directly from the "data" field (new API format)
            if let Some(data_payload) = latest_entry["data"].as_object() {
                let data_value = serde_json::Value::Object(data_payload.clone());
                let timestamp = latest_entry["timestamp"].as_str().unwrap_or("").to_string();
                
                // Try different coordinate formats
                if let Some(coords) = extract_coordinates(&data_value) {
                    return Ok(Some((coords.0, coords.1, timestamp)));
                }
            }
            // Fallback: try old format with data_payload as string
            else if let Some(payload) = latest_entry["data_payload"].as_str() {
                if let Ok(parsed_payload) = serde_json::from_str::<serde_json::Value>(payload) {
                    let timestamp = latest_entry["timestamp"].as_str().unwrap_or("").to_string();
                    
                    // Try different coordinate formats
                    if let Some(coords) = extract_coordinates(&parsed_payload) {
                        return Ok(Some((coords.0, coords.1, timestamp)));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn extract_coordinates(data: &serde_json::Value) -> Option<(f64, f64)> {
    // Try various coordinate formats
    
    // Format: {"coordinates": [lat, lng, alt?]}
    if let Some(coords) = data["coordinates"].as_array() {
        if coords.len() >= 2 {
            if let (Some(lat), Some(lng)) = (coords[0].as_f64(), coords[1].as_f64()) {
                return Some((lat, lng));
            }
        }
    }
    
    // Format: {"map": [lat, lng, alt?]}
    if let Some(map_coords) = data["map"].as_array() {
        if map_coords.len() >= 2 {
            if let (Some(lat), Some(lng)) = (map_coords[0].as_f64(), map_coords[1].as_f64()) {
                return Some((lat, lng));
            }
        }
    }
    
    // Format: {"lat": 40.7128, "lng": -74.0060}
    if let (Some(lat), Some(lng)) = (data["lat"].as_f64(), data["lng"].as_f64()) {
        return Some((lat, lng));
    }
    
    // Format: {"latitude": 40.7128, "longitude": -74.0060}
    if let (Some(lat), Some(lng)) = (data["latitude"].as_f64(), data["longitude"].as_f64()) {
        return Some((lat, lng));
    }
    
    // Format: {"gps": {"lat": 40.7128, "lon": -74.0060}}
    if let Some(gps) = data["gps"].as_object() {
        if let (Some(lat), Some(lng)) = (gps["lat"].as_f64(), gps["lon"].as_f64()) {
            return Some((lat, lng));
        }
    }
    
    None
}
