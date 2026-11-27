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

/// Hide username/password of a proxy URL so it can be logged safely.
fn mask_proxy_credentials(proxy_url: &str) -> String {
    if let Some((scheme, rest)) = proxy_url.split_once("://") {
        if let Some((_, host)) = rest.rsplit_once('@') {
            return format!("{}://***@{}", scheme, host);
        }
    }
    proxy_url.to_string()
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
    let mut client_builder = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(std::time::Duration::from_secs(cli.timeout));

    // Optional proxy for all outgoing requests to YouTube
    if let Some(proxy_url) = cli.proxy.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        let proxy = reqwest::Proxy::all(proxy_url).unwrap_or_else(|e| {
            eprintln!("Invalid proxy URL '{}': {}", mask_proxy_credentials(proxy_url), e);
            std::process::exit(1);
        });
        info!("Using proxy: {}", mask_proxy_credentials(proxy_url));
        client_builder = client_builder.proxy(proxy);
    }

    let http_client = client_builder
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

#[cfg(test)]
mod tests {
    use super::mask_proxy_credentials;

    #[test]
    fn masks_credentials() {
        assert_eq!(
            mask_proxy_credentials("socks5://user:secret@1.2.3.4:1080"),
            "socks5://***@1.2.3.4:1080"
        );
        assert_eq!(
            mask_proxy_credentials("http://proxy.local:8080"),
            "http://proxy.local:8080"
        );
    }
}
