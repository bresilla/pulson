use warp::Filter;

/// Serves `ui/dist/index.html` at `/` and all other files under `ui/dist/`.
pub fn ui_routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // GET  /         → index.html
    let index = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("pulson-ui/ui/dist/index.html"));

    // GET  /<file>   → pulson-ui/ui/dist/<file>
    let static_dir = warp::get().and(warp::fs::dir("pulson-ui/ui/dist"));

    index.or(static_dir)
}
