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
    
    Ok(())
}

pub fn get_device_data(db: &Database, device_id: &str) -> Result<Option<String>, StatusCode> {
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
        Ok(json!({
            "topic": row.get::<_, String>(0)?,
            "last_seen": row.get::<_, String>(1)?
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut topics = Vec::new();
    for topic in topic_iter {
        topics.push(topic.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    if topics.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::to_string(&topics).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
    }
}

pub fn list_all_devices(db: &Database) -> Result<Value, StatusCode> {
    let conn = db.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut stmt = conn.prepare("SELECT id, name, last_seen FROM devices ORDER BY last_seen DESC")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let device_iter = stmt.query_map([], |row| {
        Ok(json!({
            "device_id": row.get::<_, String>(0)?,
            "name": row.get::<_, Option<String>>(1)?,
            "last_seen": row.get::<_, String>(2)?
        }))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut devices = Vec::new();
    for device in device_iter {
        devices.push(device.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    }
    
    Ok(json!(devices))
}

pub fn list_user_devices(db: &Database, username: &str) -> Result<Value, StatusCode> {
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
        let last_seen = row.get::<_, String>(2)?;
        
        // Strip username prefix from device_id for display
        let display_device_id = &full_device_id[user_prefix.len()..];
        
        Ok(json!({
            "device_id": display_device_id,
            "last_seen": last_seen
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