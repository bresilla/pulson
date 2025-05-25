use sled::Db;
use std::sync::Arc;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Rejection;

use crate::logic::serve::auth::Unauthorized; // To use the same rejection type

const TOKEN_PREFIX: &str = "token:";

/// Generates a new UUID token, stores it with the associated username, and returns the token.
pub fn generate_and_store_token(db: &Arc<Db>, username: &str) -> Result<String, StatusCode> {
    let token = Uuid::new_v4().to_string();
    let token_key = format!("{}{}", TOKEN_PREFIX, token);
    db.insert(token_key.as_bytes(), username.as_bytes())
        .map_err(|e| {
            eprintln!("Failed to store token for user {}: {}", username, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(token)
}

/// Validates a token string and returns the associated username if valid.
/// Otherwise, returns a Rejection.
pub fn validate_token(db: &Arc<Db>, token_str: &str) -> Result<String, Rejection> {
    let token_key = format!("{}{}", TOKEN_PREFIX, token_str);
    match db.get(token_key.as_bytes()) {
        Ok(Some(username_bytes)) => {
            String::from_utf8(username_bytes.to_vec()).map_err(|e| {
                eprintln!("Failed to decode username for token {}: {}", token_str, e);
                warp::reject::custom(Unauthorized)
            })
        }
        Ok(None) => Err(warp::reject::custom(Unauthorized)), // Token not found
        Err(e) => {
            eprintln!("Database error validating token {}: {}", token_str, e);
            Err(warp::reject::custom(Unauthorized)) // DB error, treat as unauthorized
        }
    }
}

/// (Optional) Function to remove a token, e.g., for logout.
/// Returns true if the token was found and removed, false otherwise.
pub fn revoke_token(db: &Arc<Db>, token_str: &str) -> Result<bool, StatusCode> {
    let token_key = format!("{}{}", TOKEN_PREFIX, token_str);
    match db.remove(token_key.as_bytes()) {
        Ok(Some(_)) => Ok(true), // Token found and removed
        Ok(None) => Ok(false),    // Token not found
        Err(e) => {
            eprintln!("Database error revoking token {}: {}", token_str, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
