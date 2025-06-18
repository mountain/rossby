//! rossby - A blazingly fast, in-memory, NetCDF-to-API server
//!
//! This is the main entry point for the rossby application.

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

use rossby::data_loader::load_netcdf;
use rossby::handlers::{image_handler, metadata_handler, point_handler};
use rossby::{AppState, Config, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let (config, netcdf_path) = Config::load()?;

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    info!("Starting rossby v{}", env!("CARGO_PKG_VERSION"));
    info!("Loading NetCDF file: {:?}", netcdf_path);

    // Load NetCDF data
    let (metadata, data) = load_netcdf(&netcdf_path)?;

    info!("Found {} variables", metadata.variables.len());
    info!("Found {} dimensions", metadata.dimensions.len());

    // Create application state
    let state = AppState::new(config.clone(), metadata, data);

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
    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| rossby::RossbyError::Server {
                message: format!("Failed to bind to address: {}", e),
            })?;

    axum::serve(listener, app)
        .await
        .map_err(|e| rossby::RossbyError::Server {
            message: format!("Server error: {}", e),
        })?;

    Ok(())
}
