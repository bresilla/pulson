
use serde::{Deserialize, Serialize};

/// Data types that can be stored in the database and transmitted via REST API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    /// Simple ping/heartbeat with no data payload
    Pulse,
    /// GPS coordinates with latitude, longitude, and optional altitude
    GPS { lat: f64, lon: f64, alt: Option<f64> },
    /// Sensor reading with a numeric value
    Sensor { value: f64 },
    /// Digital trigger or switch state
    Trigger { state: bool },
    /// Text-based event or message
    Event { message: String },
    /// Image data with dimensions and pixel data
    Image { 
        rows: u32, 
        cols: u32, 
        channels: u32, 
        data: Vec<u8> 
    },
}

impl DataType {
    /// Get the type name as a string for database storage
    pub fn type_name(&self) -> &'static str {
        match self {
            DataType::Pulse => "pulse",
            DataType::GPS { .. } => "gps",
            DataType::Sensor { .. } => "sensor",
            DataType::Trigger { .. } => "trigger",
            DataType::Event { .. } => "event",
            DataType::Image { .. } => "image",
        }
    }

    /// Attempt to parse JSON data into a DataType
    pub fn from_json(value: &serde_json::Value, topic: &str) -> Option<Self> {
        match value {
            // Handle null values as Pulse
            serde_json::Value::Null => Some(DataType::Pulse),
            
            // Handle numbers as Sensor readings
            serde_json::Value::Number(n) => {
                if let Some(val) = n.as_f64() {
                    Some(DataType::Sensor { value: val })
                } else {
                    None
                }
            },
            
            // Handle booleans as Triggers
            serde_json::Value::Bool(b) => Some(DataType::Trigger { state: *b }),
            
            // Handle strings as Events
            serde_json::Value::String(s) => Some(DataType::Event { message: s.clone() }),
            
            // Handle arrays - could be GPS coordinates or other data
            serde_json::Value::Array(arr) => {
                // Check if it looks like GPS coordinates
                if Self::is_gps_topic(topic) && arr.len() >= 2 && arr.len() <= 3 {
                    if let (Some(lat), Some(lon)) = (arr[0].as_f64(), arr[1].as_f64()) {
                        let alt = if arr.len() == 3 { arr[2].as_f64() } else { None };
                        return Some(DataType::GPS { lat, lon, alt });
                    }
                }
                None
            },
            
            // Handle objects
            serde_json::Value::Object(obj) => {
                // Check for ping patterns
                if obj.is_empty() || (obj.len() == 1 && obj.contains_key("ping")) {
                    return Some(DataType::Pulse);
                }
                
                // Check for nested GPS object pattern like {"GPS": {"lat": ..., "lon": ..., "alt": ...}}
                if let Some(gps_obj) = obj.get("GPS").and_then(|v| v.as_object()) {
                    if let Some(gps) = Self::parse_gps_object(gps_obj) {
                        return Some(gps);
                    }
                }
                
                // Check for GPS object patterns directly in the object
                if Self::is_gps_topic(topic) {
                    if let Some(gps) = Self::parse_gps_object(obj) {
                        return Some(gps);
                    }
                }
                
                // Check for nested image object pattern like {"image": {"rows": ..., "cols": ..., ...}}
                if let Some(image_obj) = obj.get("image").and_then(|v| v.as_object()) {
                    if let Some(image) = Self::parse_image_object(image_obj) {
                        return Some(image);
                    }
                }
                
                // Check for image object patterns directly in the object
                if Self::is_image_topic(topic) {
                    if let Some(image) = Self::parse_image_object(obj) {
                        return Some(image);
                    }
                }
                
                // Check for sensor object patterns
                if let Some(sensor) = Self::parse_sensor_object(obj) {
                    return Some(sensor);
                }
                
                // Check for trigger object patterns
                if let Some(trigger) = Self::parse_trigger_object(obj) {
                    return Some(trigger);
                }
                
                // Check for event object patterns (should come last as fallback)
                if let Some(event) = Self::parse_event_object(obj) {
                    return Some(event);
                }
                
                None
            }
        }
    }

    /// Check if topic name suggests GPS data
    fn is_gps_topic(topic: &str) -> bool {
        let topic_lower = topic.to_lowercase();
        topic_lower.contains("gps") || 
        topic_lower.contains("location") || 
        topic_lower.contains("coordinates") || 
        topic_lower.contains("position") ||
        topic_lower.contains("lat") ||
        topic_lower.contains("lon")
    }

    /// Check if topic name suggests image data
    fn is_image_topic(topic: &str) -> bool {
        let topic_lower = topic.to_lowercase();
        topic_lower.contains("image") || 
        topic_lower.contains("img") || 
        topic_lower.contains("photo") || 
        topic_lower.contains("camera") ||
        topic_lower.contains("picture")
    }

    /// Parse GPS data from object
    fn parse_gps_object(obj: &serde_json::Map<String, serde_json::Value>) -> Option<DataType> {
        // Try different common GPS object formats
        let lat = obj.get("lat")
            .or_else(|| obj.get("latitude"))
            .or_else(|| obj.get("y"))
            .and_then(|v| v.as_f64())?;
            
        let lon = obj.get("lon")
            .or_else(|| obj.get("lng"))
            .or_else(|| obj.get("longitude"))
            .or_else(|| obj.get("x"))
            .and_then(|v| v.as_f64())?;
            
        let alt = obj.get("alt")
            .or_else(|| obj.get("altitude"))
            .or_else(|| obj.get("z"))
            .and_then(|v| v.as_f64());

        Some(DataType::GPS { lat, lon, alt })
    }

    /// Parse sensor data from object
    fn parse_sensor_object(obj: &serde_json::Map<String, serde_json::Value>) -> Option<DataType> {
        let value = obj.get("value")
            .or_else(|| obj.get("reading"))
            .or_else(|| obj.get("measurement"))
            .or_else(|| obj.get("sensor"))
            .and_then(|v| v.as_f64())?;
            
        Some(DataType::Sensor { value })
    }

    /// Parse trigger data from object
    fn parse_trigger_object(obj: &serde_json::Map<String, serde_json::Value>) -> Option<DataType> {
        let state = obj.get("state")
            .or_else(|| obj.get("trigger"))
            .or_else(|| obj.get("active"))
            .or_else(|| obj.get("enabled"))
            .and_then(|v| v.as_bool())?;
            
        Some(DataType::Trigger { state })
    }

    /// Parse image data from object
    fn parse_image_object(obj: &serde_json::Map<String, serde_json::Value>) -> Option<DataType> {
        let rows = obj.get("rows")
            .or_else(|| obj.get("height"))
            .and_then(|v| v.as_u64())? as u32;
            
        let cols = obj.get("cols")
            .or_else(|| obj.get("width"))
            .and_then(|v| v.as_u64())? as u32;
            
        let channels = obj.get("channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32; // Default to 3 channels (RGB)
            
        let data = obj.get("data")
            .and_then(|v| v.as_array())?
            .iter()
            .filter_map(|v| v.as_u64().map(|n| n as u8))
            .collect::<Vec<u8>>();

        // Validate data size matches dimensions
        if data.len() == (rows * cols * channels) as usize {
            Some(DataType::Image { rows, cols, channels, data })
        } else {
            None
        }
    }

    /// Parse event data from object
    fn parse_event_object(obj: &serde_json::Map<String, serde_json::Value>) -> Option<DataType> {
        let message = obj.get("event")
            .or_else(|| obj.get("message"))
            .or_else(|| obj.get("msg"))
            .and_then(|v| v.as_str())?;
            
        Some(DataType::Event { message: message.to_string() })
    }

    /// Serialize to JSON for database storage
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Parse from JSON string (from database)
    pub fn from_json_string(json_str: &str) -> Option<Self> {
        serde_json::from_str(json_str).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pulse_detection() {
        assert_eq!(
            DataType::from_json(&json!(null), "heartbeat"),
            Some(DataType::Pulse)
        );
        
        assert_eq!(
            DataType::from_json(&json!({}), "ping"),
            Some(DataType::Pulse)
        );
        
        assert_eq!(
            DataType::from_json(&json!({"ping": null}), "status"),
            Some(DataType::Pulse)
        );
    }

    #[test]
    fn test_gps_detection() {
        // Array format
        assert_eq!(
            DataType::from_json(&json!([40.7128, -74.0060]), "gps_location"),
            Some(DataType::GPS { lat: 40.7128, lon: -74.0060, alt: None })
        );
        
        // Object format
        assert_eq!(
            DataType::from_json(&json!({"lat": 40.7128, "lon": -74.0060}), "coordinates"),
            Some(DataType::GPS { lat: 40.7128, lon: -74.0060, alt: None })
        );
    }

    #[test]
    fn test_sensor_detection() {
        assert_eq!(
            DataType::from_json(&json!(23.5), "temperature"),
            Some(DataType::Sensor { value: 23.5 })
        );
        
        assert_eq!(
            DataType::from_json(&json!({"value": 75.2}), "humidity"),
            Some(DataType::Sensor { value: 75.2 })
        );
    }

    #[test]
    fn test_trigger_detection() {
        assert_eq!(
            DataType::from_json(&json!(true), "switch"),
            Some(DataType::Trigger { state: true })
        );
        
        assert_eq!(
            DataType::from_json(&json!({"state": false}), "door"),
            Some(DataType::Trigger { state: false })
        );
    }

    #[test]
    fn test_event_detection() {
        assert_eq!(
            DataType::from_json(&json!("System started"), "log"),
            Some(DataType::Event { message: "System started".to_string() })
        );
    }

    #[test]
    fn test_type_names() {
        assert_eq!(DataType::Pulse.type_name(), "pulse");
        assert_eq!(DataType::GPS { lat: 0.0, lon: 0.0, alt: None }.type_name(), "gps");
        assert_eq!(DataType::Sensor { value: 0.0 }.type_name(), "sensor");
        assert_eq!(DataType::Trigger { state: true }.type_name(), "trigger");
        assert_eq!(DataType::Event { message: "test".to_string() }.type_name(), "event");
        assert_eq!(DataType::Image { rows: 10, cols: 10, channels: 3, data: vec![] }.type_name(), "image");
    }
}
