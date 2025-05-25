use serde_json::json;
use sled::Db;
use std::sync::Arc;
use warp::http::StatusCode;

/// Represents the data needed to create a new user.
pub struct NewUser<'a> {
    pub username: &'a str,
    pub hashed_password: &'a str,
    pub role: &'a str,
}

/// Creates a new user in the database.
pub fn create_user(db: &Arc<Db>, user_data: NewUser) -> Result<(), StatusCode> {
    let user_key = format!("user:{}", user_data.username);
    if db.contains_key(user_key.as_bytes()).unwrap_or(false) {
        return Err(StatusCode::CONFLICT);
    }
    db.insert(user_key.as_bytes(), user_data.hashed_password.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let role_key = format!("role:{}", user_data.username);
    db.insert(role_key.as_bytes(), user_data.role.as_bytes())
        .map_err(|_| {
            // Attempt to clean up the user key if role insertion fails
            let _ = db.remove(user_key.as_bytes());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(())
}

/// Deletes a user from the database, only if the caller is a root user.
pub fn delete_user_by_admin(db: &Arc<Db>, target_username: &str, caller_username: &str) -> Result<(), StatusCode> {
    let caller_role_key = format!("role:{}", caller_username);
    match db.get(caller_role_key.as_bytes()) {
        Ok(Some(role_bytes)) if role_bytes == b"root" => {
            let user_key = format!("user:{}", target_username);
            let role_key = format!("role:{}", target_username);
            db.remove(user_key.as_bytes()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            db.remove(role_key.as_bytes()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(())
        }
        Ok(Some(_)) | Ok(None) => Err(StatusCode::FORBIDDEN), // Caller is not root or role not found
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Lists all users, only if the caller is a root user.
/// Returns a JSON Value representing the list of users or an error StatusCode.
pub fn list_all_users_by_admin(db: &Arc<Db>, caller_username: &str) -> Result<serde_json::Value, StatusCode> {
    let caller_role_key = format!("role:{}", caller_username);
     match db.get(caller_role_key.as_bytes()) {
        Ok(Some(role_bytes)) if role_bytes == b"root" => {
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
            Ok(serde_json::Value::Array(users))
        }
        Ok(Some(_)) | Ok(None) => Err(StatusCode::FORBIDDEN),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
