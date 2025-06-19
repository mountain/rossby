//! rossby - A blazingly fast, in-memory, NetCDF-to-API server
//!
//! This is the main entry point for the rossby application.

use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::info;

use rossby::data_loader::load_netcdf;
use rossby::handlers::{heartbeat_handler, image_handler, metadata_handler, point_handler};
// Allow unused import as we might need it later
#[allow(unused_imports)]
use rossby::{
    create_http_trace_layer, init_tracing, log_data_load_stats, log_error, log_operation_end,
    log_operation_start, Config, Result, RossbyError,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with default level first
    init_tracing("info");

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting rossby server"
    );

    // Load configuration
    let startup_time = Instant::now();
    log_operation_start("config_load", None);
    let (config, netcdf_path) = Config::load().inspect_err(|e| {
        log_error(e, "Failed to load configuration");
    })?;
    log_operation_end("config_load", startup_time, true);

    // Validate configuration
    log_operation_start("config_validation", None);
    config.validate().inspect_err(|e| {
        log_error(e, "Configuration validation failed");
    })?;
    log_operation_end("config_validation", startup_time, true);

    // Re-initialize tracing with configured level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", &config.log_level);
        init_tracing(&config.log_level);
    }

    info!(
        file_path = %netcdf_path.display(),
        "Loading NetCDF file"
    );

    // Load NetCDF data and create application state
    let data_load_time = Instant::now();
    log_operation_start("data_load", Some(&netcdf_path.to_string_lossy()));

    let app_state = load_netcdf(&netcdf_path, config.clone()).inspect_err(|e| {
        log_error(e, &format!("Failed to load NetCDF file: {:?}", netcdf_path));
    })?;

    // Validate the application state
    app_state.validate().inspect_err(|e| {
        log_error(e, "Application state validation failed");
    })?;

    // Calculate approximate memory usage
    let total_memory = app_state
        .data
        .values()
        .fold(0, |acc, arr| acc + arr.len() * 4); // 4 bytes per f32

    // Log detailed information about data
    let var_names: Vec<_> = app_state.metadata.variables.keys().collect();
    let dim_details: String = app_state
        .metadata
        .dimensions
        .iter()
        .map(|(name, dim)| format!("{}={}", name, dim.size))
        .collect::<Vec<_>>()
        .join(", ");

    log_data_load_stats(
        &netcdf_path.to_string_lossy(),
        var_names.len(),
        &var_names.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        app_state.metadata.dimensions.len(),
        &dim_details,
        total_memory,
    );

    log_operation_end("data_load", data_load_time, true);

    // Wrap in Arc for sharing
    let state = Arc::new(app_state);

    // Build the router
    let app = Router::new()
        .route("/metadata", get(metadata_handler))
        .route("/point", get(point_handler))
        .route("/image", get(image_handler))
        .route("/heartbeat", get(heartbeat_handler))
        .layer(CorsLayer::permissive())
        // Add tracing layer for request/response logging
        // Temporarily commenting out due to type issues
        // .layer(create_http_trace_layer())
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

    info!(
        address = %addr,
        "Server listening on http://{}", addr
    );

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        log_error(
            &RossbyError::Server {
                message: format!("Failed to bind to address: {}", e),
            },
            &format!("Failed to bind to address: {}", addr),
        );
        RossbyError::Server {
            message: format!("Failed to bind to address: {}", e),
        }
    })?;

    // Set up graceful shutdown
    let shutdown_future = shutdown_signal();

    info!(
        host = %config.server.host,
        port = config.server.port,
        workers = ?config.server.workers,
        "Server is ready to accept connections"
    );

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
            info!(
                signal = "SIGINT",
                "Received Ctrl+C, starting graceful shutdown"
            );
        },
        _ = terminate => {
            info!(
                signal = "SIGTERM",
                "Received SIGTERM, starting graceful shutdown"
            );
        },
    }
}
