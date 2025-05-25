use crate::database::{Database, PasswordManager};
use anyhow::Result;
use chrono::{DateTime, Utc};
use limbo::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Root,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Root => "root",
            UserRole::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "root" => Ok(UserRole::Root),
            "user" => Ok(UserRole::User),
            _ => Err(anyhow::anyhow!("Invalid user role: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<UserRole>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
            last_login: user.last_login,
            is_active: user.is_active,
        }
    }
}

impl Database {
    /// Create a new user with hashed password
    pub async fn create_user(&self, username: &str, password: &str, role: UserRole) -> Result<User> {
        // Validate password strength
        PasswordManager::validate_password_strength(password)?;
        
        // Hash the password
        let password_hash = PasswordManager::hash_password(password)?;
        
        // Check if username already exists
        if self.user_exists(username).await? {
            return Err(anyhow::anyhow!("Username '{}' already exists", username));
        }

        self.execute(
            "INSERT INTO users (username, password_hash, role) VALUES (?1, ?2, ?3)",
            params!(username, password_hash, role.as_str()),
        ).await?;

        // Return the created user
        self.get_user_by_username(username).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created user"))
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        self.query_row(
            "SELECT id, username, password_hash, role, created_at, updated_at, last_login, is_active 
             FROM users WHERE username = ?1 AND is_active = 1",
            params!(username),
            |row| {
                Ok(User {
                    id: row.get::<i64>(0)?,
                    username: row.get::<String>(1)?,
                    password_hash: row.get::<String>(2)?,
                    role: UserRole::from_str(&row.get::<String>(3)?).unwrap_or(UserRole::User),
                    created_at: row.get::<DateTime<Utc>>(4)?,
                    updated_at: row.get::<DateTime<Utc>>(5)?,
                    last_login: row.get::<Option<DateTime<Utc>>>(6)?,
                    is_active: row.get::<bool>(7)?,
                })
            },
        ).await
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: i64) -> Result<Option<User>> {
        self.query_row(
            "SELECT id, username, password_hash, role, created_at, updated_at, last_login, is_active 
             FROM users WHERE id = ?1 AND is_active = 1",
            params!(user_id),
            |row| {
                Ok(User {
                    id: row.get::<i64>(0)?,
                    username: row.get::<String>(1)?,
                    password_hash: row.get::<String>(2)?,
                    role: UserRole::from_str(&row.get::<String>(3)?).unwrap_or(UserRole::User),
                    created_at: row.get::<DateTime<Utc>>(4)?,
                    updated_at: row.get::<DateTime<Utc>>(5)?,
                    last_login: row.get::<Option<DateTime<Utc>>>(6)?,
                    is_active: row.get::<bool>(7)?,
                })
            },
        ).await
    }

    /// Check if a username already exists
    pub async fn user_exists(&self, username: &str) -> Result<bool> {
        let count = self.query_row(
            "SELECT COUNT(*) FROM users WHERE username = ?1",
            params!(username),
            |row| Ok(row.get::<i64>(0)?),
        ).await?;
        
        Ok(count.unwrap_or(0) > 0)
    }

    /// Authenticate user with username and password
    pub async fn authenticate_user(&self, username: &str, password: &str) -> Result<Option<User>> {
        if let Some(user) = self.get_user_by_username(username).await? {
            if PasswordManager::verify_password(password, &user.password_hash)? {
                // Update last login time
                self.update_user_last_login(user.id).await?;
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    /// Update user's last login time
    pub async fn update_user_last_login(&self, user_id: i64) -> Result<()> {
        self.execute(
            "UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE id = ?1",
            params!(user_id),
        ).await?;
        Ok(())
    }

    /// List all active users (admin only)
    pub async fn list_users(&self) -> Result<Vec<UserInfo>> {
        self.query_rows(
            "SELECT id, username, role, created_at, updated_at, last_login, is_active 
             FROM users WHERE is_active = 1 ORDER BY created_at DESC",
            params!(),
            |row| {
                Ok(UserInfo {
                    id: row.get::<i64>(0)?,
                    username: row.get::<String>(1)?,
                    role: UserRole::from_str(&row.get::<String>(2)?).unwrap_or(UserRole::User),
                    created_at: row.get::<DateTime<Utc>>(3)?,
                    last_login: row.get::<Option<DateTime<Utc>>>(5)?,
                    is_active: row.get::<bool>(6)?,
                })
            },
        ).await
    }

    /// Deactivate a user (soft delete)
    pub async fn deactivate_user(&self, username: &str) -> Result<bool> {
        self.execute(
            "UPDATE users SET is_active = 0 WHERE username = ?1 AND is_active = 1",
            params!(username),
        ).await?;
        
        Ok(true) // Limbo doesn't return affected rows count easily, so we assume success
    }

    /// Change user password
    pub async fn change_user_password(&self, user_id: i64, new_password: &str) -> Result<()> {
        // Validate new password strength
        PasswordManager::validate_password_strength(new_password)?;
        
        // Hash the new password
        let password_hash = PasswordManager::hash_password(new_password)?;
        
        self.execute(
            "UPDATE users SET password_hash = ?1 WHERE id = ?2",
            params!(password_hash, user_id),
        ).await?;
        
        Ok(())
    }

    /// Update user role (root only)
    pub async fn update_user_role(&self, username: &str, new_role: UserRole) -> Result<bool> {
        self.execute(
            "UPDATE users SET role = ?1 WHERE username = ?2 AND is_active = 1",
            params!(new_role.as_str(), username),
        ).await?;
        
        Ok(true)
    }

    /// Check if user has root privileges
    pub async fn is_root_user(&self, user_id: i64) -> Result<bool> {
        if let Some(user) = self.get_user_by_id(user_id).await? {
            Ok(user.role == UserRole::Root)
        } else {
            Ok(false)
        }
    }

    /// Get user count
    pub async fn get_user_count(&self) -> Result<i64> {
        let count = self.query_row(
            "SELECT COUNT(*) FROM users WHERE is_active = 1",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?;
        
        Ok(count.unwrap_or(0))
    }

    /// Check if any root users exist
    pub async fn has_root_users(&self) -> Result<bool> {
        let count = self.query_row(
            "SELECT COUNT(*) FROM users WHERE role = 'root' AND is_active = 1",
            params!(),
            |row| Ok(row.get::<i64>(0)?),
        ).await?;
        
        Ok(count.unwrap_or(0) > 0)
    }
}