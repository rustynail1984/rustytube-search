mod handlers;
mod models;
mod youtube;

use axum::{routing::get, Router};
use clap::Parser;
use std::env;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use handlers::{handle_search, health_check};
use models::{AppState, Cli};

fn get_port(cli_port: u16) -> u16 {
    // Priority: PORT env var > CLI argument > default
    if let Ok(port_str) = env::var("PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            return port;
        }
    }
    cli_port
}

#[tokio::main]
async fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize tracing with log level
    // Priority: RUST_LOG env var > CLI argument > default
    let log_level = if env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap()
    } else {
        format!(
            "rustytube={},tower_http={}",
            cli.log_level.as_str(),
            cli.log_level.as_str()
        )
        .into()
    };

    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("Starting RustyTube v{}", env!("CARGO_PKG_VERSION"));

    // Create shared HTTP client with connection pooling and configurable timeout
    let http_client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(std::time::Duration::from_secs(cli.timeout))
        .build()
        .expect("Failed to create HTTP client");

    let state = Arc::new(AppState { http_client });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/search/:query", get(handle_search))
        .layer(cors)
        .with_state(state);

    let port = get_port(cli.port);

    // Parse host address
    let host: std::net::IpAddr = cli.host.parse().expect("Invalid host address");

    let addr = std::net::SocketAddr::new(host, port);

    info!("RustyTube listening on http://{}", addr);
    info!("Try: http://localhost:{}/api/search/sia", port);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.unwrap();
}
