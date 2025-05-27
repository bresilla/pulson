pub mod account_routes;
pub mod device_routes;
pub mod password_utils;
pub mod user_management;
pub mod token_service; // Add this line

use crate::logic::serve::api::account_routes::{delete_user, list_users, login, register, user_info}; // Added user_info
// use crate::logic::serve::api::device_routes::{list_all, list_one, ping, delete_device};
use crate::logic::serve::database::Database;
use warp::Filter;

/// Compose all account- and device-related routes into one API filter.
pub fn api_routes(
    db: Database,
    root_pass: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let reg = register(db.clone(), root_pass.clone());
    let log = login(db.clone());
    let logout_route = crate::logic::serve::api::account_routes::logout(db.clone()); // Add logout
    let del = delete_user(db.clone());
    let list = list_users(db.clone());
    let userinfo_route = user_info(db.clone()); // Add userinfo route

    let p = device_routes::ping(db.clone());
    let la = device_routes::list_all(db.clone());
    let lo = device_routes::list_one(db.clone());
    let dd = device_routes::delete_device(db.clone()); // Add delete_device route

    // Routes already include /api prefix in their individual definitions
    reg.or(log).or(logout_route).or(del).or(list).or(userinfo_route).or(p).or(lo).or(la).or(dd)
}
