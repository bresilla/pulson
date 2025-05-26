use crate::logic::serve::database::{Database, store_token, get_username_by_token, revoke_token as db_revoke_token};
use uuid::Uuid;
use warp::http::StatusCode;
use warp::Rejection;

use crate::logic::serve::auth::Unauthorized; // To use the same rejection type

/// Generates a new UUID token, stores it with the associated username, and returns the token.
pub fn generate_and_store_token(db: &Database, username: &str) -> Result<String, StatusCode> {
    let token = Uuid::new_v4().to_string();
    store_token(db, &token, username)?;
    Ok(token)
}

/// Validates a token string and returns the associated username if valid.
/// Otherwise, returns a Rejection.
pub fn validate_token(db: &Database, token_str: &str) -> Result<String, Rejection> {
    match get_username_by_token(db, token_str) {
        Ok(Some(username)) => Ok(username),
        Ok(None) => Err(warp::reject::custom(Unauthorized)), // Token not found
        Err(_) => {
            eprintln!("Database error validating token {}", token_str);
            Err(warp::reject::custom(Unauthorized)) // DB error, treat as unauthorized
        }
    }
}

/// Function to remove a token, e.g., for logout.
/// Returns true if the token was found and removed, false otherwise.
pub fn revoke_token(db: &Database, token_str: &str) -> Result<bool, StatusCode> {
    db_revoke_token(db, token_str)
}
