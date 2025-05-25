use crate::logic::serve::auth::authenticated_user;
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::Deserialize;
use serde_json::json;
use sled::Db;
use std::sync::Arc;
use uuid::Uuid;
use warp::{
    body::json as warp_body_json,
    http::StatusCode,
    reply::{json as warp_json, with_status},
    Filter, Rejection,
};

#[derive(Deserialize)]
struct AccountPayload {
    username: String,
    password: String,
    rootpass: Option<String>,
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
            let user_key = format!("user:{}", payload.username);
            if db.contains_key(user_key.as_bytes()).unwrap_or(false) {
                return StatusCode::CONFLICT;
            }

            // Hash the password before storing
            let hashed_password = match hash(&payload.password, DEFAULT_COST) {
                Ok(h) => h,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR, // Or a more specific error
            };

            let _ = db.insert(user_key.as_bytes(), hashed_password.as_bytes());

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
            let role_key = format!("role:{}", payload.username);
            let _ = db.insert(role_key.as_bytes(), role.as_bytes());
            StatusCode::CREATED
        })
}

/// POST /account/login
pub fn login(db: Arc<Db>) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("account" / "login"))
        .and(warp::body::json())
        .map(move |payload: AccountPayload| {
            let user_key = format!("user:{}", payload.username);
            let err = || {
                with_status(
                    warp_json(&json!({ "error": "invalid credentials" })),
                    StatusCode::UNAUTHORIZED,
                )
            };

            match db
                .get(user_key.as_bytes())
                .ok()
                .flatten()
                .map(|v| v.to_vec())
            {
                Some(stored_hashed_password_bytes) => {
                    // Convert stored bytes to string to verify
                    let stored_hashed_password = match String::from_utf8(stored_hashed_password_bytes) {
                        Ok(s) => s,
                        Err(_) => return err(), // Or a more specific error if password wasn't valid UTF-8
                    };

                    match verify(&payload.password, &stored_hashed_password) {
                        Ok(true) => {
                            let token = Uuid::new_v4().to_string();
                            let tok_key = format!("token:{}", token);
                            let _ = db.insert(tok_key.as_bytes(), payload.username.as_bytes());
                            // Store role with token for easier lookup, or fetch role separately
                            // For now, keeping it simple as original
                            with_status(warp_json(&json!({ "token": token })), StatusCode::OK)
                        }
                        Ok(false) => err(),
                        Err(_) => {
                            // bcrypt verify can error out for various reasons e.g. invalid hash format
                            // log this error server-side
                            eprintln!("Error verifying password for user {}", payload.username);
                            err()
                        }
                    }
                }
                _ => err(),
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
        .map(move |target: String, caller: String| {
            let role_key = format!("role:{}", caller);
            if let Ok(Some(role)) = db.get(role_key.as_bytes()) {
                if &*role == b"root" {
                    let _ = db.remove(format!("user:{}", target).as_bytes());
                    let _ = db.remove(format!("role:{}", target).as_bytes());
                    return StatusCode::OK;
                }
            }
            StatusCode::FORBIDDEN
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
        .map(move |caller: String| {
            let role_key = format!("role:{}", caller);
            let is_root = db
                .get(role_key.as_bytes())
                .ok()
                .flatten()
                .map(|v| v.as_ref() == b"root")
                .unwrap_or(false);
            if !is_root {
                return with_status(
                    warp_json(&json!({ "error": "forbidden" })),
                    StatusCode::FORBIDDEN,
                );
            }

            let mut users = Vec::new();
            for item in db.iter().flatten() {
                let key = String::from_utf8_lossy(&item.0);
                if let Some(name) = key.strip_prefix("user:") {
                    let role = db
                        .get(format!("role:{}", name).as_bytes())
                        .ok()
                        .flatten()
                        .and_then(|v| String::from_utf8(v.to_vec()).ok())
                        .unwrap_or_else(|| "user".into());
                    users.push(json!({ "username": name, "role": role }));
                }
            }
            with_status(warp_json(&users), StatusCode::OK)
        })
}
