use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;
use serde_json::{json, Value};

pub type Database = Arc<Mutex<Connection>>;

pub fn init_database<P: AsRef<Path>>(db_path: P) -> anyhow::Result<Database> {
    let conn = Connection::open(db_path)?;
    
    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            username TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tokens (
            token TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(username) REFERENCES users(username) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            name TEXT,
            last_seen DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS topics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL,
            topic TEXT NOT NULL,
            last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id) ON DELETE CASCADE,
            UNIQUE(device_id, topic)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_config (
            username TEXT PRIMARY KEY,
            online_threshold_seconds INTEGER NOT NULL DEFAULT 30,
            warning_threshold_seconds INTEGER NOT NULL DEFAULT 300,
            stale_threshold_seconds INTEGER NOT NULL DEFAULT 3600,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(username) REFERENCES users(username) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create table for historical pulse data
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pulse_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL,
            topic TEXT NOT NULL,
            timestamp DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create table for structured data storage
    conn.execute(
        "CREATE TABLE IF NOT EXISTS device_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL,
            topic TEXT NOT NULL,
            data_type TEXT NOT NULL,
            data_payload TEXT NOT NULL,
            timestamp DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(device_id) REFERENCES devices(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_pulse_history_device_timestamp 
         ON pulse_history(device_id, timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_pulse_history_topic_timestamp 
         ON pulse_history(device_id, topic, timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_device_data_device_timestamp 
         ON device_data(device_id, timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_device_data_type_timestamp 
         ON device_data(device_id, data_type, timestamp)",
        [],
    )?;

    Ok(Arc::new(Mutex::new(conn)))
}

// User management functions
pub fn create_user(db: &Database, username: &str, password_hash: &str, role: &str) -> Result<(), StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match conn.execute(
        "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3)",
        [username, password_hash, role],
    ) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
            Err(StatusCode::CONFLICT) // User already exists
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn get_user_password_hash(db: &Database, username: &str) -> Result<Option<String>, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stmt = conn.prepare("SELECT password_hash FROM users WHERE username = ?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut rows = stmt.query_map([username], |row| {
        Ok(row.get::<_, String>(0)?)
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match rows.next() {
        Some(Ok(password_hash)) => Ok(Some(password_hash)),
        Some(Err(_)) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        None => Ok(None),
    }
}

pub fn get_user_role(db: &Database, username: &str) -> Result<Option<String>, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stmt = conn.prepare("SELECT role FROM users WHERE username = ?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut rows = stmt.query_map([username], |row| {
        Ok(row.get::<_, String>(0)?)
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match rows.next() {
        Some(Ok(role)) => Ok(Some(role)),
        Some(Err(_)) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        None => Ok(None),
    }
}

pub fn delete_user(db: &Database, username: &str) -> Result<bool, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match conn.execute("DELETE FROM users WHERE username = ?1", [username]) {
        Ok(0) => Ok(false), // No user was deleted
        Ok(_) => Ok(true),  // User was deleted
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn list_all_users(db: &Database) -> Result<Value, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stmt = conn.prepare("SELECT username, role, created_at FROM users ORDER BY username")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let user_iter = stmt.query_map([], |row| {
        Ok(json!({
            "username": row.get::<_, String>(0)?,
            "role": row.get::<_, String>(1)?,
            "created_at": row.get::<_, String>(2)?
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut users = Vec::new();
    for user in user_iter {
        users.push(user.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    Ok(json!({ "users": users }))
}

// Token management functions
pub fn store_token(db: &Database, token: &str, username: &str) -> Result<(), StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match conn.execute(
        "INSERT INTO tokens (token, username) VALUES (?1, ?2)",
        [token, username],
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn get_username_by_token(db: &Database, token: &str) -> Result<Option<String>, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stmt = conn.prepare("SELECT username FROM tokens WHERE token = ?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut rows = stmt.query_map([token], |row| {
        Ok(row.get::<_, String>(0)?)
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match rows.next() {
        Some(Ok(username)) => Ok(Some(username)),
        Some(Err(_)) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        None => Ok(None),
    }
}

pub fn revoke_token(db: &Database, token: &str) -> Result<bool, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match conn.execute("DELETE FROM tokens WHERE token = ?1", [token]) {
        Ok(0) => Ok(false), // No token was deleted
        Ok(_) => Ok(true),  // Token was deleted
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// User configuration management functions
pub fn get_user_config(db: &Database, username: &str) -> Result<Option<crate::logic::config::StatusConfig>, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stmt = conn.prepare("SELECT online_threshold_seconds, warning_threshold_seconds, stale_threshold_seconds FROM user_config WHERE username = ?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut rows = stmt.query_map([username], |row| {
        Ok(crate::logic::config::StatusConfig {
            online_threshold_seconds: row.get::<_, u64>(0)?,
            warning_threshold_seconds: row.get::<_, u64>(1)?,
            stale_threshold_seconds: row.get::<_, u64>(2)?,
        })
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match rows.next() {
        Some(Ok(config)) => Ok(Some(config)),
        Some(Err(_)) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        None => Ok(None),
    }
}

pub fn set_user_config(db: &Database, username: &str, config: &crate::logic::config::StatusConfig) -> Result<(), StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Validate thresholds
    if config.online_threshold_seconds >= config.warning_threshold_seconds {
        return Err(StatusCode::BAD_REQUEST);
    }
    if config.warning_threshold_seconds >= config.stale_threshold_seconds {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    match conn.execute(
        "INSERT INTO user_config (username, online_threshold_seconds, warning_threshold_seconds, stale_threshold_seconds, updated_at) 
         VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
         ON CONFLICT(username) DO UPDATE SET 
         online_threshold_seconds = excluded.online_threshold_seconds,
         warning_threshold_seconds = excluded.warning_threshold_seconds,
         stale_threshold_seconds = excluded.stale_threshold_seconds,
         updated_at = excluded.updated_at",
        [
            username,
            &config.online_threshold_seconds.to_string(),
            &config.warning_threshold_seconds.to_string(),
            &config.stale_threshold_seconds.to_string(),
        ],
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn get_user_config_or_default(db: &Database, username: &str) -> crate::logic::config::StatusConfig {
    match get_user_config(db, username) {
        Ok(Some(config)) => config,
        Ok(None) | Err(_) => crate::logic::config::StatusConfig::default(),
    }
}

// Device management functions
pub fn store_device_data(db: &Database, device_id: &str, name: Option<&str>, data: &str, timestamp: &str) -> Result<(), StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Parse the data to extract topic information
    let parsed_data: serde_json::Value = serde_json::from_str(data)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let topic = parsed_data.get("topic")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    // Handle device insertion/update more carefully
    if let Some(device_name) = name {
        // If a name is provided, insert new device or update both name and timestamp
        conn.execute(
            "INSERT INTO devices (id, name, last_seen) VALUES (?1, ?2, ?3) 
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, last_seen = excluded.last_seen",
            [device_id, device_name, timestamp],
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        // If no name is provided, only update timestamp, preserve existing name
        conn.execute(
            "INSERT INTO devices (id, name, last_seen) VALUES (?1, NULL, ?2) 
             ON CONFLICT(id) DO UPDATE SET last_seen = excluded.last_seen",
            [device_id, timestamp],
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    // Then, insert or update the topic record
    conn.execute(
        "INSERT INTO topics (device_id, topic, last_seen) VALUES (?1, ?2, ?3) 
         ON CONFLICT(device_id, topic) DO UPDATE SET last_seen = excluded.last_seen",
        [device_id, topic, timestamp],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Store historical pulse data
    conn.execute(
        "INSERT INTO pulse_history (device_id, topic, timestamp) VALUES (?1, ?2, ?3)",
        [device_id, topic, timestamp],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(())
}

pub fn get_device_data(db: &Database, device_id: &str, status_config: &crate::logic::config::StatusConfig) -> Result<Option<String>, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Check if device exists
    let mut device_stmt = conn.prepare("SELECT id FROM devices WHERE id = ?1")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let device_exists = device_stmt.query_map([device_id], |_| Ok(()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .next()
        .is_some();
    
    if !device_exists {
        return Ok(None);
    }
    
    // Get all topics for this device
    let mut topics_stmt = conn.prepare("SELECT topic, last_seen FROM topics WHERE device_id = ?1 ORDER BY last_seen DESC")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let topic_iter = topics_stmt.query_map([device_id], |row| {
        let topic_name = row.get::<_, String>(0)?;
        let last_seen_str = row.get::<_, String>(1)?;
        let last_seen = chrono::DateTime::parse_from_rfc3339(&last_seen_str)
            .map_err(|_| rusqlite::Error::FromSqlConversionFailure(
                1, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid timestamp format"))
            ))?
            .with_timezone(&chrono::Utc);
        
        let status = status_config.calculate_topic_status(&last_seen);
        
        Ok((topic_name, last_seen_str, status))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut topics = Vec::new();
    for topic_result in topic_iter {
        let (topic_name, last_seen_str, status) = topic_result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        // Get data types for this topic from device_data table
        let mut data_types_stmt = conn.prepare(
            "SELECT DISTINCT data_type FROM device_data WHERE device_id = ?1 AND topic = ?2"
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let data_types_iter = data_types_stmt.query_map([device_id, &topic_name], |row| {
            Ok(row.get::<_, String>(0)?)
        }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let mut data_types = vec!["PULSE".to_string()]; // Always include PULSE since it's in topics table
        for data_type_result in data_types_iter {
            let data_type = data_type_result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            data_types.push(data_type);
        }
        
        topics.push(json!({
            "topic": topic_name,
            "last_seen": last_seen_str,
            "status": status,
            "data_types": data_types
        }));
    }
    
    if topics.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::to_string(&topics).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
    }
}

pub fn list_user_devices(db: &Database, username: &str, status_config: &crate::logic::config::StatusConfig) -> Result<Value, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let user_prefix = format!("{}:", username);
    let mut stmt = conn.prepare("
        SELECT d.id, d.name, 
               COALESCE(MAX(t.last_seen), d.last_seen) as last_activity
        FROM devices d
        LEFT JOIN topics t ON d.id = t.device_id
        WHERE d.id LIKE ?1
        GROUP BY d.id, d.name, d.last_seen
        ORDER BY last_activity DESC
    ").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let device_iter = stmt.query_map([format!("{}%", user_prefix)], |row| {
        let full_device_id = row.get::<_, String>(0)?;
        let last_seen_str = row.get::<_, String>(2)?;
        
        // Strip username prefix from device_id for display
        let display_device_id = &full_device_id[user_prefix.len()..];
        
        // Parse timestamp and calculate status
        let last_seen = match chrono::DateTime::parse_from_rfc3339(&last_seen_str) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => return Err(rusqlite::Error::FromSqlConversionFailure(
                2, rusqlite::types::Type::Text, Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid timestamp format"))
            )),
        };
        
        let status = status_config.calculate_device_status(&last_seen);
        
        Ok(json!({
            "device_id": display_device_id,
            "last_seen": last_seen_str,
            "status": status
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut devices = Vec::new();
    for device in device_iter {
        devices.push(device.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    Ok(json!(devices))
}

/// Deletes a device from the database.
pub fn delete_device(db: &Database, device_id: &str) -> Result<bool, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Delete device - topics will be deleted automatically due to CASCADE
    let rows_affected = conn.execute(
        "DELETE FROM devices WHERE id = ?1",
        [device_id],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(rows_affected > 0)
}

/// Get historical pulse data for visualization
pub fn get_pulse_history(db: &Database, device_id: &str, topic: Option<&str>, time_range: &str) -> Result<Value, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Calculate time range
    let now = chrono::Utc::now();
    let start_time = match time_range {
        "1h" => now - chrono::Duration::hours(1),
        "1d" => now - chrono::Duration::days(1),
        "1w" => now - chrono::Duration::weeks(1),
        "1m" => now - chrono::Duration::days(30),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Build query based on whether we want specific topic or all topics
    let (query, params): (String, Vec<String>) = if let Some(topic_name) = topic {
        (
            "SELECT 
                datetime(timestamp) as time,
                COUNT(*) as pulse_count
             FROM pulse_history 
             WHERE device_id = ?1 AND topic = ?2 AND timestamp >= ?3
             GROUP BY strftime('%Y-%m-%d %H:%M', timestamp)
             ORDER BY timestamp".to_string(),
            vec![device_id.to_string(), topic_name.to_string(), start_time.to_rfc3339()]
        )
    } else {
        (
            "SELECT 
                datetime(timestamp) as time,
                topic,
                COUNT(*) as pulse_count
             FROM pulse_history 
             WHERE device_id = ?1 AND timestamp >= ?2
             GROUP BY strftime('%Y-%m-%d %H:%M', timestamp), topic
             ORDER BY timestamp, topic".to_string(),
            vec![device_id.to_string(), start_time.to_rfc3339()]
        )
    };
    
    let mut stmt = conn.prepare(&query)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let pulse_iter = stmt.query_map(params_refs.as_slice(), |row| {
        if topic.is_some() {
            Ok(json!({
                "time": row.get::<_, String>(0)?,
                "pulse_count": row.get::<_, i64>(1)?
            }))
        } else {
            Ok(json!({
                "time": row.get::<_, String>(0)?,
                "topic": row.get::<_, String>(1)?,
                "pulse_count": row.get::<_, i64>(2)?
            }))
        }
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut pulses = Vec::new();
    for pulse in pulse_iter {
        pulses.push(pulse.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    Ok(json!({
        "time_range": time_range,
        "start_time": start_time.to_rfc3339(),
        "end_time": now.to_rfc3339(),
        "data": pulses
    }))
}

/// Get pulse statistics for a device
pub fn get_pulse_stats(db: &Database, device_id: &str, time_range: &str) -> Result<Value, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Calculate time range
    let now = chrono::Utc::now();
    let start_time = match time_range {
        "1h" => now - chrono::Duration::hours(1),
        "1d" => now - chrono::Duration::days(1),
        "1w" => now - chrono::Duration::weeks(1),
        "1m" => now - chrono::Duration::days(30),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    
    let mut stmt = conn.prepare(
        "SELECT 
            topic,
            COUNT(*) as total_pulses,
            MIN(timestamp) as first_pulse,
            MAX(timestamp) as last_pulse
         FROM pulse_history 
         WHERE device_id = ?1 AND timestamp >= ?2
         GROUP BY topic
         ORDER BY total_pulses DESC"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let stats_iter = stmt.query_map([device_id, &start_time.to_rfc3339()], |row| {
        Ok(json!({
            "topic": row.get::<_, String>(0)?,
            "total_pulses": row.get::<_, i64>(1)?,
            "first_pulse": row.get::<_, String>(2)?,
            "last_pulse": row.get::<_, String>(3)?
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stats_data = Vec::new();
    for stat in stats_iter {
        stats_data.push(stat.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    // Get total count for the device
    let mut total_stmt = conn.prepare(
        "SELECT COUNT(*) FROM pulse_history WHERE device_id = ?1 AND timestamp >= ?2"
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let total_count: i64 = total_stmt.query_row([device_id, &start_time.to_rfc3339()], |row| {
        Ok(row.get::<_, i64>(0)?)
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(json!({
        "time_range": time_range,
        "start_time": start_time.to_rfc3339(),
        "end_time": now.to_rfc3339(),
        "total_pulses": total_count,
        "stats": stats_data // Changed from "topics" to "stats"
    }))
}

/// Store structured data for a device
pub fn store_device_data_payload(
    db: &Database, 
    device_id: &str, 
    name: Option<&str>, 
    topic: &str,
    data_type: &str,
    data_payload: &serde_json::Value,
    timestamp: &str
) -> Result<(), StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Handle device insertion/update
    if let Some(device_name) = name {
        conn.execute(
            "INSERT INTO devices (id, name, last_seen) VALUES (?1, ?2, ?3) 
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, last_seen = excluded.last_seen",
            [device_id, device_name, timestamp],
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        conn.execute(
            "INSERT INTO devices (id, name, last_seen) VALUES (?1, NULL, ?2) 
             ON CONFLICT(id) DO UPDATE SET last_seen = excluded.last_seen",
            [device_id, timestamp],
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    // Insert or update the topic record
    conn.execute(
        "INSERT INTO topics (device_id, topic, last_seen) VALUES (?1, ?2, ?3) 
         ON CONFLICT(device_id, topic) DO UPDATE SET last_seen = excluded.last_seen",
        [device_id, topic, timestamp],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Store the structured data
    conn.execute(
        "INSERT INTO device_data (device_id, topic, data_type, data_payload, timestamp) 
         VALUES (?1, ?2, ?3, ?4, ?5)",
        [
            device_id, 
            topic, 
            data_type, 
            &data_payload.to_string(), 
            timestamp
        ],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(())
}

/// Get latest data for a device and topic
pub fn get_device_latest_data(
    db: &Database, 
    device_id: &str, 
    topic: Option<&str>, 
    data_type: Option<&str>
) -> Result<Value, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let (query, params): (String, Vec<String>) = match (topic, data_type) {
        (Some(t), Some(dt)) => (
            "SELECT topic, data_type, data_payload, timestamp 
             FROM device_data 
             WHERE device_id = ?1 AND topic = ?2 AND data_type = ?3 
             ORDER BY timestamp DESC 
             LIMIT 10".to_string(),
            vec![device_id.to_string(), t.to_string(), dt.to_string()]
        ),
        (Some(t), None) => (
            "SELECT topic, data_type, data_payload, timestamp 
             FROM device_data 
             WHERE device_id = ?1 AND topic = ?2 
             ORDER BY timestamp DESC 
             LIMIT 10".to_string(),
            vec![device_id.to_string(), t.to_string()]
        ),
        (None, Some(dt)) => (
            "SELECT topic, data_type, data_payload, timestamp 
             FROM device_data 
             WHERE device_id = ?1 AND data_type = ?2 
             ORDER BY timestamp DESC 
             LIMIT 10".to_string(),
            vec![device_id.to_string(), dt.to_string()]
        ),
        (None, None) => (
            "SELECT topic, data_type, data_payload, timestamp 
             FROM device_data 
             WHERE device_id = ?1 
             ORDER BY timestamp DESC 
             LIMIT 10".to_string(),
            vec![device_id.to_string()]
        ),
    };
    
    let mut stmt = conn.prepare(&query)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let data_iter = stmt.query_map(params_refs.as_slice(), |row| {
        let data_payload_str: String = row.get(2)?;
        let data_payload: serde_json::Value = serde_json::from_str(&data_payload_str)
            .map_err(|_| rusqlite::Error::FromSqlConversionFailure(
                2, rusqlite::types::Type::Text, 
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid JSON"))
            ))?;
        
        Ok(json!({
            "topic": row.get::<_, String>(0)?,
            "data_type": row.get::<_, String>(1)?,
            "data": data_payload,
            "timestamp": row.get::<_, String>(3)?
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut data_records = Vec::new();
    for record in data_iter {
        data_records.push(record.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    Ok(json!({
        "device_id": device_id,
        "data": data_records
    }))
}