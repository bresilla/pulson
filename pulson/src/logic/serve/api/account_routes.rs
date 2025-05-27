use crate::logic::serve::api::password_utils::{hash_password, verify_password};
use crate::logic::serve::api::user_management::{create_user, delete_user_by_admin, list_all_users_by_admin, NewUser};
use crate::logic::serve::api::token_service::{generate_and_store_token, revoke_token};
use crate::logic::serve::database::{Database, get_user_password_hash, get_user_role};
use crate::logic::serve::auth::authenticated_user;
use serde::Deserialize;
use serde_json::json;
use warp::{
    body::json as warp_body_json,
    header::optional,
    http::StatusCode,
    reply::{json as warp_json, with_status},
    Filter, Rejection,
};

#[derive(serde::Serialize)]
struct UserInfoResponse {
    username: String,
    is_root: bool,
}

#[derive(Deserialize)]
struct AccountPayload {
    username: String,
    password: String,
    rootpass: Option<String>,
}

#[derive(Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

/// POST /api/account/register
pub fn register(
    db: Database,
    root_pass: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("api" / "account" / "register"))
        .and(warp_body_json())
        .map(move |payload: AccountPayload| {
            let hashed_password = match hash_password(&payload.password) {
                Ok(h) => h,
                Err(status_code) => {
                    return with_status(
                        warp_json(&json!({ "error": "password hashing failed" })),
                        status_code,
                    )
                }
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
                Ok(_) => with_status(
                    warp_json(&json!({ "message": "user created successfully" })),
                    StatusCode::CREATED,
                ),
                Err(status_code) => with_status(
                    warp_json(&json!({ "error": "user creation failed" })),
                    status_code,
                ),
            }
        })
}

/// POST /api/account/login
pub fn login(db: Database) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path!("api" / "account" / "login"))
        .and(warp::body::json()) // Expect LoginPayload
        .map(move |payload: LoginPayload| { // Use LoginPayload here
            let err = || {
                with_status(
                    warp_json(&json!({ "error": "invalid credentials" })),
                    StatusCode::UNAUTHORIZED,
                )
            };

            match get_user_password_hash(&db, &payload.username) {
                Ok(Some(stored_hashed_password)) => {
                    match verify_password(&payload.password, &stored_hashed_password) {
                        Ok(true) => {
                            match generate_and_store_token(&db, &payload.username) {
                                Ok(token) => {
                                    with_status(warp_json(&json!({ "token": token })), StatusCode::OK)
                                }
                                Err(status_code) => {
                                    with_status(warp_json(&json!({ "error": "login failed" })), status_code)
                                }
                            }
                        }
                        Ok(false) => err(),
                        Err(_status_code) => err(), // Prefixed status_code with _
                    }
                }
                Ok(None) => err(),
                Err(_) => err(),
            }
        })
}

/// DELETE /api/account/{username}
pub fn delete_user(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::delete()
        .and(warp::path!("api" / "account" / String))
        .and(auth)
        .map(move |target_username: String, caller_username: String| {
            match delete_user_by_admin(&db, &target_username, &caller_username) {
                Ok(_) => with_status(
                    warp_json(&json!({ "message": "user deleted successfully" })),
                    StatusCode::OK,
                ),
                Err(status_code) => with_status(
                    warp_json(&json!({ "error": "user deletion failed" })),
                    status_code,
                ),
            }
        })
}

/// GET /api/account/users
pub fn list_users(
    db: Database,
) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "account" / "users"))
        .and(auth)
        .map(move |caller_username: String| {
            match list_all_users_by_admin(&db, &caller_username) {
                Ok(users_json) => with_status(warp_json(&users_json), StatusCode::OK),
                Err(status_code) => with_status(
                    warp_json(&json!({ "error": "forbidden or error" })),
                    status_code,
                ),
            }
        })
}

/// POST /api/account/logout
pub fn logout(db: Database) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::post()
        .and(warp::path!("api" / "account" / "logout"))
        .and(auth)
        .and(optional::<String>("authorization"))
        .map(move |_username: String, auth_header: Option<String>| { // Prefixed username with _
            if let Some(header) = auth_header {
                if let Some(token_str) = header.strip_prefix("Bearer ") {
                    match revoke_token(&db, token_str) {
                        Ok(true) => {
                            return with_status(
                                warp_json(&json!({ "message": "logged out" })),
                                StatusCode::OK,
                            );
                        }
                        Ok(false) => {
                            return with_status(
                                warp_json(&json!({ "error": "logout failed, token not found" })),
                                StatusCode::BAD_REQUEST,
                            );
                        }
                        Err(status_code) => {
                            return with_status(
                                warp_json(&json!({ "error": "logout failed" })),
                                status_code,
                            );
                        }
                    }
                }
            }
            with_status(
                warp_json(&json!({ "error": "invalid token format" })),
                StatusCode::BAD_REQUEST,
            )
        })
}

/// GET /api/userinfo
pub fn user_info(db: Database) -> impl Filter<Extract = impl warp::Reply, Error = Rejection> + Clone {
    let auth = authenticated_user(db.clone());
    warp::get()
        .and(warp::path!("api" / "userinfo"))
        .and(auth)
        .map(move |username: String| {
            match get_user_role(&db, &username) {
                Ok(Some(role)) => {
                    let is_root = role == "root";
                    let user_info_response = UserInfoResponse { username, is_root };
                    with_status(warp_json(&user_info_response), StatusCode::OK)
                }
                Ok(None) => with_status(
                    warp_json(&json!({ "error": "user role not found" })),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ),
                Err(_) => with_status(
                    warp_json(&json!({ "error": "database error" })),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ),
            }
        })
}
