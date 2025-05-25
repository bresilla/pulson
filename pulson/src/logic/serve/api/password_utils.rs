use bcrypt::{hash as bcrypt_hash, verify as bcrypt_verify, DEFAULT_COST};
use warp::http::StatusCode;

/// Hashes a password using bcrypt.
pub fn hash_password(password: &str) -> Result<String, StatusCode> {
    bcrypt_hash(password, DEFAULT_COST).map_err(|e| {
        eprintln!("Password hashing failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// Verifies a password against a stored hash.
pub fn verify_password(password: &str, hashed_password_str: &str) -> Result<bool, StatusCode> {
    bcrypt_verify(password, hashed_password_str).map_err(|e| {
        eprintln!("Password verification failed: {}", e);
        // Potentially return UNAUTHORIZED or a more generic error
        // depending on how this is used in login flows.
        // For now, aligning with previous logic of returning err() which leads to UNAUTHORIZED.
        StatusCode::UNAUTHORIZED 
    })
}
