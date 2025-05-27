pub mod account_routes;
pub mod device_routes;
pub mod password_utils;
pub mod user_management;
pub mod token_service; // Add this line

use crate::logic::serve::api::account_routes::{delete_user, list_users, login, register, user_info}; // Added user_info
// use crate::logic::serve::api::device_routes::{list_all, list_one, ping, delete_device};
use crate::logic::serve::database::Database;
use crate::logic::config::StatusConfig;
use std::sync::{Arc, Mutex};
use warp::Filter;

/// Compose all account- and device-related routes into one API filter.
pub fn api_routes(
    db: Database,
    root_pass: Option<String>,
    status_config: Arc<Mutex<StatusConfig>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let reg = register(db.clone(), root_pass.clone());
    let log = login(db.clone());
    let logout_route = crate::logic::serve::api::account_routes::logout(db.clone()); // Add logout
    let del = delete_user(db.clone());
    let list = list_users(db.clone());
    let userinfo_route = user_info(db.clone()); // Add userinfo route

    let p = device_routes::ping(db.clone());
    let data_route = device_routes::data(db.clone()); // Add data route
    let la = device_routes::list_all(db.clone());
    let lo = device_routes::list_one(db.clone());
    let dd = device_routes::delete_device(db.clone()); // Add delete_device route
    // config_reload route removed - no longer needed with purely server-based configuration
    let config_get = device_routes::get_config(status_config.clone()); // Add config get route
    let config_update = device_routes::update_config(status_config.clone(), db.clone()); // Add config update route
    let user_config_get = device_routes::get_user_config(db.clone()); // Add user config get route
    let user_config_set = device_routes::set_user_config(db.clone()); // Add user config set route
    let device_history = device_routes::get_device_history(db.clone()); // Add pulse history route
    let device_stats = device_routes::get_device_stats(db.clone()); // Add pulse stats route
    let device_data_latest = device_routes::get_device_data_latest(db.clone()); // Add device data route

    // Routes already include /api prefix in their individual definitions
    reg.or(log).or(logout_route).or(del).or(list).or(userinfo_route).or(p).or(data_route).or(lo).or(la).or(dd).or(config_get).or(config_update).or(user_config_get).or(user_config_set).or(device_history).or(device_stats).or(device_data_latest)
}
