use mime_guess;
use rust_embed::RustEmbed;
use warp::{http::Response, Filter, Rejection};

#[derive(RustEmbed)]
#[folder = "../pulson-ui/ui/dist"]
struct Asset;

/// Serves static assets under `/static/<path>` and falls back to `index.html` for SPA.
pub fn ui_routes() -> impl Filter<Extract = (impl warp::Reply,), Error = Rejection> + Clone {
    // 1) static files
    let static_files = warp::get()
        .and(warp::path("static"))
        .and(warp::path::tail())
        .map(|tail: warp::path::Tail| {
            let path = tail.as_str();
            if let Some(content) = Asset::get(path) {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header("content-type", mime.as_ref())
                    .body(content.data.into_owned())
            } else {
                Response::builder().status(404).body("Not Found".into())
            }
        });

    // 2) SPA fallback
    let spa = warp::get().and(warp::path::full()).map(|_| {
        let file = Asset::get("index.html").expect("index.html missing");
        Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(file.data.into_owned())
    });

    static_files.or(spa)
}
