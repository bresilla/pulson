use std::sync::Arc;
use warp::{header::optional, reject::Reject, Filter, Rejection};

/// Marker for unauthorized rejection
#[derive(Debug)]
pub struct Unauthorized;
impl Reject for Unauthorized {}

/// A filter that extracts `Authorization: Bearer <token>` and looks up the username in sled.
pub fn authenticated_user(
    db: Arc<sled::Db>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    optional::<String>("authorization").and_then(move |auth: Option<String>| {
        let db = db.clone();
        async move {
            let header = auth.ok_or_else(|| warp::reject::custom(Unauthorized))?;
            let token = header
                .strip_prefix("Bearer ")
                .ok_or_else(|| warp::reject::custom(Unauthorized))?;
            let key = format!("token:{}", token);
            match db.get(key.as_bytes()).ok().flatten() {
                Some(bytes) => {
                    let user = String::from_utf8(bytes.to_vec())
                        .map_err(|_| warp::reject::custom(Unauthorized))?;
                    Ok(user)
                }
                None => Err(warp::reject::custom(Unauthorized)),
            }
        }
    })
}
