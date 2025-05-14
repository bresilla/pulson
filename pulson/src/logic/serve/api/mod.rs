pub mod account_routes;
pub mod device_routes;

use crate::logic::serve::api::account_routes::{delete_user, list_users, login, register};
use crate::logic::serve::api::device_routes::{list_all, list_one, ping};
use std::sync::Arc;
use warp::Filter;

/// Compose all account- and device-related routes into one API filter.
pub fn api_routes(
    db: Arc<sled::Db>,
    root_pass: Option<String>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let reg = register(db.clone(), root_pass.clone());
    let log = login(db.clone());
    let del = delete_user(db.clone());
    let list = list_users(db.clone());

    let p = ping(db.clone());
    let la = list_all(db.clone());
    let lo = list_one(db.clone());

    reg.or(log).or(del).or(list).or(p).or(lo).or(la)
}
