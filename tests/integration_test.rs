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
static TEST_ADDR: OnceCell<SocketAddr> = OnceCell::new();
static TEST_TEMP_DIR: OnceCell<tempfile::TempDir> = OnceCell::new();
static TEST_FILE_PATH: OnceCell<String> = OnceCell::new();

/// Start a test server on a specified port
async fn start_test_server() -> SocketAddr {
    // Initialize test data and get temp directory
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

    // Use port 0 to let the OS assign an available port
    let addr = SocketAddr::from((std::net::Ipv4Addr::new(127, 0, 0, 1), 0));

    // Start by creating a listener to get the OS-assigned port
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port");

    // Get the actual bound address with the OS-assigned port
    let bound_addr = listener.local_addr().expect("Failed to get local address");

    // Get the test file path
    let file_path = TEST_FILE_PATH.get().expect("Test file path not set");

    // Start the server
    tokio::spawn(async move {
        // Create a minimal config
        let config = rossby::Config {
            server: rossby::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: bound_addr.port(),
                workers: Some(1),
            },
            ..Default::default()
        };

        // Load the test NetCDF file
        let app_state =
            rossby::data_loader::load_netcdf(std::path::Path::new(file_path), config.clone())
                .expect("Failed to load test NetCDF file");

        let state = std::sync::Arc::new(app_state);

        // Create the router
        let app = axum::Router::new()
            .route(
                "/metadata",
                axum::routing::get(rossby::handlers::metadata_handler),
            )
            .route(
                "/point",
                axum::routing::get(rossby::handlers::point_handler),
            )
            .route(
                "/image",
                axum::routing::get(rossby::handlers::image_handler),
            )
            .layer(tower_http::cors::CorsLayer::permissive())
            .with_state(state);

        println!("Test server started on {}", bound_addr);

        axum::serve(listener, app).await.expect("Server error");
    });

    // The server will take some time to start up fully
    println!("Test server starting on {}", bound_addr);

    bound_addr
}

/// Initialize a new test server for each test
async fn init_test_environment() -> SocketAddr {
    // Always start a new server for each test
    let server_addr = start_test_server().await;

    println!(
        "Test server started, waiting for it to be ready at {}",
        server_addr
    );

    // Wait for the server to be ready
    let mut retries = 10;
    while retries > 0 {
        match reqwest::Client::new()
            .get(format!("http://{}/metadata", server_addr))
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
        {
            Ok(_) => {
                println!("Server is ready at {}", server_addr);
                break; // Server is ready
            }
            Err(e) => {
                // Wait and retry
                println!(
                    "Server not ready, retrying... ({} retries left): {}",
                    retries, e
                );
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                retries -= 1;
            }
        }
    }

    if retries == 0 {
        panic!("Server did not become ready in time at {}", server_addr);
    }

    server_addr
}

#[tokio::test]
async fn test_server_startup() {
    // Ensure server is running
    let addr = init_test_environment().await;

    // Just verify we have a valid port (non-zero)
    assert!(addr.port() > 0);
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

    let body = response.text().await.expect("Failed to get response body");

    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

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

    println!("Using server address for point endpoint test: {}", addr);

    // Test nearest neighbor interpolation (using coordinates within our test data bounds)
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&time_index=0&vars=temperature&interpolation=nearest",
    )
    .await
    .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to get response body");

    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

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

    let body = response.text().await.expect("Failed to get response body");

    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

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

    let body = response.text().await.expect("Failed to get response body");

    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

    assert!(json.get("error").is_some());
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("outside the range"));
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
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "image/jpeg"
    );

    let bytes = response
        .bytes()
        .await
        .expect("Failed to get response bytes");

    // Verify it's a valid JPEG
    assert!(image_utils::detect_image_format(&bytes).unwrap() == image::ImageFormat::Jpeg);

    // Test error case - invalid variable
    let response = http_client::get(&addr, "/image?var=nonexistent&time_index=0")
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 400);
}
