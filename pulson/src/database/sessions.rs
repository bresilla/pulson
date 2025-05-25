use crate::database::Database;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use limbo::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub token: String,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Database {
    /// Create a new session for a user
    pub async fn create_session(&self, user_id: i64, duration_hours: i64) -> Result<Session> {
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::hours(duration_hours);

        self.execute(
            "INSERT INTO sessions (token, user_id, created_at, expires_at, last_used) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params!(token, user_id, now, expires_at, now),
        ).await?;

        Ok(Session {
            id: 0, // We'll get this from the database in a real implementation
            token,
            user_id,
            created_at: now,
            expires_at,
            last_used: now,
            is_active: true,
        })
    }

    /// Get session by token if valid and active
    pub async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>> {
        self.query_row(
            "SELECT id, token, user_id, created_at, expires_at, last_used, is_active 
             FROM sessions 
             WHERE token = ?1 AND is_active = 1 AND expires_at > CURRENT_TIMESTAMP",
            params!(token),
            |row| {
                Ok(Session {
                    id: row.get::<i64>(0)?,
                    token: row.get::<String>(1)?,
                    user_id: row.get::<i64>(2)?,
                    created_at: row.get::<DateTime<Utc>>(3)?,
                    expires_at: row.get::<DateTime<Utc>>(4)?,
                    last_used: row.get::<DateTime<Utc>>(5)?,
                    is_active: row.get::<bool>(6)?,
                })
            },
        ).await
    }

    /// Update session's last used time
    pub async fn update_session_last_used(&self, token: &str) -> Result<()> {
        self.execute(
            "UPDATE sessions SET last_used = CURRENT_TIMESTAMP 
             WHERE token = ?1 AND is_active = 1",
            params!(token),
        ).await?;
        Ok(())
    }

    /// Invalidate a specific session
    pub async fn invalidate_session(&self, token: &str) -> Result<bool> {
        self.execute(
            "UPDATE sessions SET is_active = 0 WHERE token = ?1",
            params!(token),
        ).await?;
        
        Ok(true)
    }

    /// Invalidate all sessions for a user
    pub async fn invalidate_user_sessions(&self, user_id: i64) -> Result<()> {
        self.execute(
            "UPDATE sessions SET is_active = 0 WHERE user_id = ?1 AND is_active = 1",
            params!(user_id),
        ).await?;
        
        Ok(())
    }

    /// Get all active sessions for a user
    pub async fn get_user_sessions(&self, user_id: i64) -> Result<Vec<SessionInfo>> {
        self.query_rows(
            "SELECT token, expires_at, created_at 
             FROM sessions 
             WHERE user_id = ?1 AND is_active = 1 AND expires_at > CURRENT_TIMESTAMP
             ORDER BY created_at DESC",
            params!(user_id),
            |row| {
                Ok(SessionInfo {
                    token: row.get::<String>(0)?,
                    expires_at: row.get::<DateTime<Utc>>(1)?,
                    created_at: row.get::<DateTime<Utc>>(2)?,
                })
            },
        ).await
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<()> {
        self.execute(
            "DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP",
            params!(),
        ).await?;
        
        Ok(())
    }

    /// Clean up old inactive sessions (older than 30 days)
    pub async fn cleanup_old_sessions(&self) -> Result<()> {
        self.execute(
            "DELETE FROM sessions 
             WHERE is_active = 0 AND created_at < datetime('now', '-30 days')",
            params!(),
        ).await?;
        
        Ok(())
    }

    /// Extend session expiration time
    pub async fn extend_session(&self, token: &str, additional_hours: i64) -> Result<bool> {
        self.execute(
            "UPDATE sessions 
             SET expires_at = datetime(expires_at, '+' || ?1 || ' hours'),
                 last_used = CURRENT_TIMESTAMP
             WHERE token = ?2 AND is_active = 1",
            params!(additional_hours, token),
        ).await?;
        
        Ok(true)
    }

    /// Get session count for a user
    pub async fn get_user_session_count(&self, user_id: i64) -> Result<i64> {
        let count = self.query_row(
            "SELECT COUNT(*) FROM sessions 
             WHERE user_id = ?1 AND is_active = 1 AND expires_at > CURRENT_TIMESTAMP",
            params!(user_id),
            |row| Ok(row.get::<i64>(0)?),
        ).await?;
        
        Ok(count.unwrap_or(0))
    }

    /// Get total active session count
    pub async fn get_active_session_count(&self) -> Result<i64> {
        let count = self.query_row(
            "SELECT COUNT(*) FROM sessions 
             WHERE is_active = 1 AND expires_at > CURRENT_TIMESTAMP",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?;
        
        Ok(count.unwrap_or(0))
    }

    /// Validate token and return associated user ID
    pub async fn validate_token(&self, token: &str) -> Result<Option<i64>> {
        if let Some(session) = self.get_session_by_token(token).await? {
            // Update last used time
            self.update_session_last_used(token).await?;
            Ok(Some(session.user_id))
        } else {
            Ok(None)
        }
    }
}