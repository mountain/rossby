//! rossby - A blazingly fast, in-memory, NetCDF-to-API server
//!
//! This is the main entry point for the rossby application.

use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use rossby::data_loader::load_netcdf;
use rossby::handlers::{image_handler, metadata_handler, point_handler};
use rossby::{Config, Result, RossbyError};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default level first
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting rossby v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let (config, netcdf_path) = Config::load().map_err(|e| {
        error!("Configuration error: {}", e);
        e
    })?;

    // Validate configuration
    config.validate().map_err(|e| {
        error!("Invalid configuration: {}", e);
        e
    })?;

    // Re-initialize tracing with configured level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", &config.log_level);
    }

    info!("Loading NetCDF file: {:?}", netcdf_path);

    // Load NetCDF data and create application state
    let app_state = load_netcdf(&netcdf_path, config.clone()).map_err(|e| {
        error!("Failed to load NetCDF file: {}", e);
        e
    })?;

    // Validate the application state
    app_state.validate().map_err(|e| {
        error!("Invalid application state: {}", e);
        e
    })?;

    info!("Found {} variables", app_state.metadata.variables.len());
    info!("Found {} dimensions", app_state.metadata.dimensions.len());

    // Wrap in Arc for sharing
    let state = Arc::new(app_state);

    // Build the router
    let app = Router::new()
        .route("/metadata", get(metadata_handler))
        .route("/point", get(point_handler))
        .route("/image", get(image_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Create the server address
    let addr = SocketAddr::from((
        config
            .server
            .host
            .parse::<std::net::IpAddr>()
            .map_err(|e| rossby::RossbyError::Config {
                message: format!("Invalid host address: {}", e),
            })?,
        config.server.port,
    ));

    info!("Server listening on http://{}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| RossbyError::Server {
            message: format!("Failed to bind to address: {}", e),
        })?;

    // Set up graceful shutdown
    let shutdown_future = shutdown_signal();

    info!("Server is ready to accept connections");

    // Start the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_future)
        .await
        .map_err(|e| RossbyError::Server {
            message: format!("Server error: {}", e),
        })?;

    info!("Server has been gracefully shut down");
    Ok(())
}

/// Wait for a shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}
