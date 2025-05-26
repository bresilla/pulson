pub mod api;
pub mod auth;
pub mod database;
pub mod ui;

use crate::logic::serve::api::api_routes;
use crate::logic::serve::auth::Unauthorized;
use crate::logic::serve::database::init_database;
use crate::logic::serve::ui::ui_routes;
use daemonize::Daemonize;
use shellexpand;
use std::net::IpAddr;
use warp::{Filter, Rejection};

pub async fn run(
    host: String,
    port: u16,
    db_path: String,
    daemon: bool,
    root_pass: Option<String>,
    _webui: bool,
) -> anyhow::Result<()> {
    // 1) Daemonize if requested
    if daemon {
        Daemonize::new()
            .pid_file("pulson.pid")
            .chown_pid_file(false)
            .working_directory(".")
            .start()?;
    }

    // 2) Initialize SQLite database (expanding ~)
    let expanded = shellexpand::tilde(&db_path).into_owned();
    let db_file = if expanded.ends_with(".db") { 
        expanded 
    } else { 
        format!("{}/pulson.db", expanded) 
    };
    let db = init_database(&db_file)?;

    // 3) Build API routes
    let api = api_routes(db.clone(), root_pass.clone())
        .recover(|err: Rejection| async move {
            if err.find::<Unauthorized>().is_some() {
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({ "error": "Unauthorized" })),
                    warp::http::StatusCode::UNAUTHORIZED,
                ))
            } else {
                Err(err)
            }
        })
        .boxed();

    // 4) Build UI routes (static + SPA)
    let ui = ui_routes().boxed();

    // 5) Combine and serve
    let routes = api.or(ui);
    println!("pulson server running on http://{}:{}", host, port);
    let ip: IpAddr = host.parse()?;
    warp::serve(routes).run((ip, port)).await;
    Ok(())
}
