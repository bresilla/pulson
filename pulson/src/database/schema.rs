use limbo::{Database, params};
use anyhow::Result;

pub fn create_tables(db: &Database) -> Result<()> {
    // Users table
    db.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('root', 'user')),
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_login DATETIME,
            is_active BOOLEAN NOT NULL DEFAULT 1
        )",
        params!(),
    )?;

    // Sessions table
    db.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token TEXT NOT NULL UNIQUE,
            user_id INTEGER NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME NOT NULL,
            last_used DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        params!(),
    )?;

    // Devices table
    db.execute(
        "CREATE TABLE IF NOT EXISTS devices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL UNIQUE,
            name TEXT,
            description TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_active BOOLEAN NOT NULL DEFAULT 1
        )",
        params!(),
    )?;

    // Topics table
    db.execute(
        "CREATE TABLE IF NOT EXISTS topics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            topic TEXT NOT NULL,
            description TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_ping DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ping_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE,
            UNIQUE(device_id, topic)
        )",
        params!(),
    )?;

    // Pings table
    db.execute(
        "CREATE TABLE IF NOT EXISTS pings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id INTEGER NOT NULL,
            topic_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            metadata TEXT,
            FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE,
            FOREIGN KEY (topic_id) REFERENCES topics(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        params!(),
    )?;

    // Create indexes for better performance
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_devices_device_id ON devices(device_id)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_topics_device_id ON topics(device_id)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_topics_last_ping ON topics(last_ping)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_pings_device_id ON pings(device_id)",
        params!(),
    )?;
    db.execute(
        "CREATE INDEX IF NOT EXISTS idx_pings_timestamp ON pings(timestamp)",
        params!(),
    )?;

    // Create triggers for automatic timestamp updates
    db.execute(
        "CREATE TRIGGER IF NOT EXISTS update_users_timestamp 
         AFTER UPDATE ON users
         BEGIN
             UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        params!(),
    )?;

    db.execute(
        "CREATE TRIGGER IF NOT EXISTS update_devices_timestamp 
         AFTER UPDATE ON devices
         BEGIN
             UPDATE devices SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        params!(),
    )?;

    db.execute(
        "CREATE TRIGGER IF NOT EXISTS update_topics_timestamp 
         AFTER UPDATE ON topics
         BEGIN
             UPDATE topics SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        params!(),
    )?;

    Ok(())
}

pub fn cleanup_old_data(db: &Database) -> Result<()> {
    // Remove expired sessions
    db.execute(
        "DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP",
        params!(),
    )?;

    // Remove old pings (keep last 30 days)
    db.execute(
        "DELETE FROM pings WHERE timestamp < datetime('now', '-30 days')",
        params!(),
    )?;

    Ok(())
}