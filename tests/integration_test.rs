//! Integration tests for rossby server
//!
//! These tests verify that the server works correctly end-to-end.

mod common;

use common::{http_client, image_utils, test_data};
use std::net::SocketAddr;
use std::sync::Once;

// Global test setup for server
use once_cell::sync::OnceCell;

static INIT: Once = Once::new();
static TEST_PORT: u16 = 9876;
static TEST_TEMP_DIR: OnceCell<tempfile::TempDir> = OnceCell::new();
static TEST_FILE_PATH: OnceCell<String> = OnceCell::new();

/// Start a test server on a dedicated port
async fn start_test_server() -> SocketAddr {
    // Bind to a specific port for testing
    let addr = ([127, 0, 0, 1], TEST_PORT).into();

    // Create test data once
    let _temp_dir = TEST_TEMP_DIR.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_weather.nc");
        test_data::create_test_weather_nc(&file_path).unwrap();

        // Save the path for later reference
        TEST_FILE_PATH
            .set(file_path.to_string_lossy().to_string())
            .unwrap();

        dir
    });

// Start the server in a background task
let file_path = TEST_FILE_PATH.get().expect("Test file path not set");
let _server_task = tokio::spawn(async move {
    // Create a minimal config
        let config = rossby::Config {
            server: rossby::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: TEST_PORT,
                workers: Some(1),
            },
            ..Default::default()
        };
    
    // Load the test NetCDF file
    let app_state = rossby::data_loader::load_netcdf(std::path::Path::new(file_path), config.clone())
        .expect("Failed to load test NetCDF file");
    
    let state = std::sync::Arc::new(app_state);
    
    // Create the router
    let app = axum::Router::new()
        .route("/metadata", axum::routing::get(rossby::handlers::metadata_handler))
        .route("/point", axum::routing::get(rossby::handlers::point_handler))
        .route("/image", axum::routing::get(rossby::handlers::image_handler))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to test port");
    
    println!("Test server started on {}", addr);
    
    axum::serve(listener, app)
        .await
        .expect("Server error");
});

// Give the server a moment to start
tokio::time::sleep(std::time::Duration::from_millis(100)).await;

println!("Test server ready on {}", addr);

    addr
}

/// Initialize the test environment once
async fn init_test_environment() -> SocketAddr {
    let addr = start_test_server().await;
    INIT.call_once(|| {
        println!("Test environment initialized with server at {}", addr);
    });
    addr
}

#[tokio::test]
async fn test_server_startup() {
    // Ensure server is running
    let addr = init_test_environment().await;

    // Test will be more substantive in Phase 7
    // For now, we just verify the port is as expected
    assert_eq!(addr.port(), TEST_PORT);
}

#[tokio::test]
async fn test_metadata_endpoint() {
    // Initialize test environment
    let addr = init_test_environment().await;

    // Make an actual HTTP request to the metadata endpoint
    let response = http_client::get(&addr, "/metadata")
        .await
        .expect("Failed to make request");
    
    assert_eq!(response.status(), 200);
    
    let body = response
        .text()
        .await
        .expect("Failed to get response body");
    
    let json: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON response");
    
    // Verify the metadata structure
    assert!(json.get("global_attributes").is_some());
    assert!(json.get("dimensions").is_some());
    assert!(json.get("variables").is_some());
    assert!(json.get("coordinates").is_some());
    
    // Verify that our test variables are present
    let variables = json.get("variables").unwrap();
    assert!(variables.get("temperature").is_some());
    assert!(variables.get("humidity").is_some());
}

#[tokio::test]
async fn test_point_endpoint() {
    // Initialize test environment
    let addr = init_test_environment().await;

    // Test nearest neighbor interpolation (using coordinates within our test data bounds)
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&time_index=0&vars=temperature&interpolation=nearest",
    )
    .await
    .expect("Failed to make request");
    
    assert_eq!(response.status(), 200);
    
    let body = response
        .text()
        .await
        .expect("Failed to get response body");
    
    let json: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON response");
    
    assert!(json.get("temperature").is_some());
    assert!(json.get("temperature").unwrap().is_number());
    
    // Test bilinear interpolation with multiple variables
    let response = http_client::get(
        &addr,
        "/point?lon=-160.0&lat=20.0&time_index=0&vars=temperature,humidity&interpolation=bilinear",
    )
    .await
    .expect("Failed to make request");
    
    assert_eq!(response.status(), 200);
    
    let body = response
        .text()
        .await
        .expect("Failed to get response body");
    
    let json: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON response");
    
    assert!(json.get("temperature").is_some());
    assert!(json.get("humidity").is_some());
    
    // Test error case - invalid coordinates (well outside the range)
    let response = http_client::get(
        &addr,
        "/point?lon=999.0&lat=999.0&time_index=0&vars=temperature",
    )
    .await
    .expect("Failed to make request");
    
    assert_eq!(response.status(), 400);
    
    let body = response
        .text()
        .await
        .expect("Failed to get response body");
    
    let json: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON response");
    
    assert!(json.get("error").is_some());
    assert!(json["error"].as_str().unwrap().contains("outside the range"));
}

#[tokio::test]
async fn test_image_endpoint() {
    // Initialize test environment
    let addr = init_test_environment().await;

    // Request a PNG image
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&format=png",
    )
    .await
    .expect("Failed to make request");
    
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");
    
    let bytes = response
        .bytes()
        .await
        .expect("Failed to get response bytes");
    
    // Verify it's a valid PNG
    assert!(image_utils::detect_image_format(&bytes).unwrap() == image::ImageFormat::Png);
    
    // Load the image and check dimensions
    let img = image::load_from_memory(&bytes).expect("Failed to load image from memory");
    assert!(image_utils::assert_image_dimensions(&img, 100, 80).is_ok());
    
    // Try with JPEG format
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&format=jpeg&colormap=plasma",
    )
    .await
    .expect("Failed to make request");
    
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/jpeg");
    
    let bytes = response
        .bytes()
        .await
        .expect("Failed to get response bytes");
    
    // Verify it's a valid JPEG
    assert!(image_utils::detect_image_format(&bytes).unwrap() == image::ImageFormat::Jpeg);
    
    // Test error case - invalid variable
    let response = http_client::get(
        &addr,
        "/image?var=nonexistent&time_index=0",
    )
    .await
    .expect("Failed to make request");
    
    assert_eq!(response.status(), 400);
}
