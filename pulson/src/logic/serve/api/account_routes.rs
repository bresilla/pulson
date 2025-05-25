use crate::logic::serve::api::password_utils::{hash_password, verify_password};
use crate::logic::serve::api::user_management::{create_user, delete_user_by_admin, list_all_users_by_admin, NewUser};
use crate::logic::serve::api::token_service::{generate_and_store_token, revoke_token};
use serde::Deserialize;
use serde_json::json;
use sled::Db;
use std::sync::Arc;

use warp::{ // Shorten warp imports
    body::json as warp_body_json,
    http::StatusCode,
    reply::{json as warp_json, with_status},
    Filter, Rejection,
    header::optional, // Add this for optional header extraction
};

use crate::logic::serve::auth::authenticated_user;

#[derive(Deserialize)]
struct AccountPayload {
    username: String,
    password: String,
    rootpass: Option<String>,
}

#[derive(Deserialize)] // Added for login specific payload if needed, or reuse AccountPayload
struct LoginPayload {
    username: String,
    password: String,
}

/// POST /account/register
pub fn register(
    db: Arc<Db>,
    root_pass: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("account" / "register"))
        .and(warp_body_json())
        .map(move |payload: AccountPayload| {
            let hashed_password = match hash_password(&payload.password) {
                Ok(h) => h,
                Err(status_code) => return status_code,
            };

            let role = if payload
                .rootpass
                .as_ref()
                .and_then(|rp| root_pass.as_ref().map(|rp2| rp == rp2))
                .unwrap_or(false)
            {
                "root"
            } else {
                "user"
            };

            let new_user_data = NewUser {
                username: &payload.username,
                hashed_password: &hashed_password,
                role,
            };

            match create_user(&db, new_user_data) {
                Ok(_) => StatusCode::CREATED,
                Err(status_code) => status_code,
            }
        })
}

/// POST /account/login
pub fn login(db: Arc<Db>) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("account" / "login"))
        .and(warp::body::json())
        .map(move |payload: LoginPayload| { // Changed to LoginPayload if it's different, or keep AccountPayload
            let user_key = format!("user:{}", payload.username);
            let err = || {
                with_status(
                    warp_json(&json!({ "error": "invalid credentials" })),
                    StatusCode::UNAUTHORIZED,
                )
            };

            match db.get(user_key.as_bytes()).ok().flatten() {
                Some(stored_hashed_password_bytes) => {
                    let stored_hashed_password_str = match String::from_utf8(stored_hashed_password_bytes.to_vec()) {
                        Ok(s) => s,
                        Err(_) => {
                            eprintln!("Stored password for user {} is not valid UTF-8", payload.username);
                            return err();
                        }
                    };

                    match verify_password(&payload.password, &stored_hashed_password_str) {
                        Ok(true) => {
                            match generate_and_store_token(&db, &payload.username) {
                                Ok(token) => {
                                    with_status(warp_json(&json!({ "token": token })), StatusCode::OK)
                                }
                                Err(status_code) => {
                                    eprintln!("Token generation failed for user: {}", payload.username);
                                    with_status(warp_json(&json!({ "error": "login failed" })), status_code)
                                }
                            }
                        }
                        Ok(false) => err(),
                        Err(status_code) => {
                            // Log specific bcrypt error on server side if needed from verify_password internal logging
                            // The status_code from verify_password is typically UNAUTHORIZED
                            with_status(warp_json(&json!({ "error": "invalid credentials" })), status_code)
                        }
                    }
                }
                None => err(), // User not found
            }
        })
}

/// DELETE /account/<username>
pub fn delete_user(
    db: Arc<Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::delete()
        .and(warp::path!("account" / String))
        .and(auth)
        .map(move |target_username: String, caller_username: String| {
            match delete_user_by_admin(&db, &target_username, &caller_username) {
                Ok(_) => StatusCode::OK,
                Err(status_code) => status_code,
            }
        })
}

/// GET /account/users
pub fn list_users(
    db: Arc<Db>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("account" / "users"))
        .and(auth)
        .map(move |caller_username: String| {
            match list_all_users_by_admin(&db, &caller_username) {
                Ok(users_json) => with_status(warp_json(&users_json), StatusCode::OK),
                Err(status_code) => {
                    with_status(warp_json(&json!({ "error": "forbidden or error" })), status_code)
                }
            }
        })
}

/// POST /account/logout
pub fn logout(db: Arc<Db>) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone()); // Reuses existing auth to get username from token
    warp::post()
        .and(warp::path!("account" / "logout"))
        .and(auth) // Ensures the user is logged in to log out
        .and(optional::<String>("authorization")) // To extract the token itself
        .map(move |username: String, auth_header: Option<String>| { // username from authenticated_user
            if let Some(header) = auth_header {
                if let Some(token_str) = header.strip_prefix("Bearer ") {
                    match revoke_token(&db, token_str) {
                        Ok(true) => {
                            println!("User {} logged out, token revoked.", username);
                            return with_status(warp_json(&json!({ "message": "logged out" })), StatusCode::OK);
                        }
                        Ok(false) => {
                            // Token not found, but user was authenticated. This case should ideally not happen
                            // if authenticated_user and revoke_token are in sync.
                            eprintln!("Logout attempt for user {} with a token that was not found for revocation.", username);
                            return with_status(warp_json(&json!({ "error": "logout failed, token not found" })), StatusCode::BAD_REQUEST);
                        }
                        Err(status_code) => {
                            eprintln!("Failed to revoke token during logout for user {}: {:?}", username, status_code);
                            return with_status(warp_json(&json!({ "error": "logout failed" })), status_code);
                        }
                    }
                }
            }
            // No Bearer token found in header, though authenticated_user should have caught this.
            // This is a fallback.
            with_status(warp_json(&json!({ "error": "invalid token format" })), StatusCode::BAD_REQUEST)
        })
}
