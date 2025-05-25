use std::sync::Arc;
use warp::{header::optional, reject::Reject, Filter, Rejection};

use crate::logic::serve::api::token_service::validate_token;

/// Marker for unauthorized rejection
#[derive(Debug)]
pub struct Unauthorized;
impl Reject for Unauthorized {}

/// A filter that extracts `Authorization: Bearer <token>` and looks up the username in sled.
pub fn authenticated_user(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    optional::<String>("authorization").and_then(move |auth_header: Option<String>| {
        let db_clone = db.clone();
        async move {
            let header = auth_header.ok_or_else(|| warp::reject::custom(Unauthorized))?;
            let token_str = header
                .strip_prefix("Bearer ")
                .ok_or_else(|| warp::reject::custom(Unauthorized))?;

            validate_token(&db_clone, token_str) // Use the new token service function
        }
    })
}
