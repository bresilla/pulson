use crate::logic::serve::database::{Database, create_user as db_create_user, delete_user, get_user_role, list_all_users};
use warp::http::StatusCode;

/// Represents the data needed to create a new user.
pub struct NewUser<'a> {
    pub username: &'a str,
    pub hashed_password: &'a str,
    pub role: &'a str,
}

/// Creates a new user in the database.
pub fn create_user(db: &Database, user_data: NewUser) -> Result<(), StatusCode> {
    db_create_user(db, user_data.username, user_data.hashed_password, user_data.role)
}

/// Deletes a user from the database, only if the caller is a root user.
pub fn delete_user_by_admin(db: &Database, target_username: &str, caller_username: &str) -> Result<(), StatusCode> {
    // Check if caller is root
    match get_user_role(db, caller_username)? {
        Some(role) if role == "root" => {
            match delete_user(db, target_username)? {
                true => Ok(()),
                false => Err(StatusCode::NOT_FOUND), // User not found
            }
        }
        Some(_) => Err(StatusCode::FORBIDDEN), // Caller is not root
        None => Err(StatusCode::FORBIDDEN), // Caller not found
    }
}

/// Lists all users, only if the caller is a root user.
/// Returns a JSON Value representing the list of users or an error StatusCode.
pub fn list_all_users_by_admin(db: &Database, caller_username: &str) -> Result<serde_json::Value, StatusCode> {
    // Check if caller is root
    match get_user_role(db, caller_username)? {
        Some(role) if role == "root" => list_all_users(db),
        Some(_) => Err(StatusCode::FORBIDDEN), // Caller is not root
        None => Err(StatusCode::FORBIDDEN), // Caller not found
    }
}
