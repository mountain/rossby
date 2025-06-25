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
                discovery_url: None,
                max_data_points: 10_000_000, // Default 10 million points
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
            .route(
                "/heartbeat",
                axum::routing::get(rossby::handlers::heartbeat_handler),
            )
            .route("/data", axum::routing::get(rossby::handlers::data_handler))
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

    // Test nearest neighbor interpolation (using updated coordinates to match fixed data)
    // Using coordinates in the 0-360 longitude system instead of -180 to 180
    let response = http_client::get(
        &addr,
        "/point?lon=190.0&lat=10.0&time_index=0&vars=temperature&interpolation=nearest",
    )
    .await
    .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to get response body");

    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

    assert!(json.get("temperature").is_some());
    assert!(json.get("temperature").unwrap().is_number());

    // Test bilinear interpolation with multiple variables (with updated coordinates)
    let response = http_client::get(
        &addr,
        "/point?lon=200.0&lat=20.0&time_index=0&vars=temperature,humidity&interpolation=bilinear",
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
async fn test_heartbeat_endpoint() {
    // Initialize test environment
    let addr = init_test_environment().await;

    // Make a request to the heartbeat endpoint
    let response = http_client::get(&addr, "/heartbeat")
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to get response body");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

    // Verify the heartbeat response structure
    assert!(json.get("server_id").is_some());
    assert!(json.get("timestamp").is_some());
    assert!(json.get("uptime_seconds").is_some());
    assert!(json.get("status").is_some());
    assert_eq!(json.get("status").unwrap().as_str().unwrap(), "healthy");

    // Verify dataset information
    assert!(json.get("dataset").is_some());
    let dataset = json.get("dataset").unwrap();
    assert!(dataset.get("variable_count").is_some());
    assert!(dataset.get("variables").is_some());
    assert!(dataset.get("dimension_count").is_some());
    assert!(dataset.get("dimensions").is_some());
    assert!(dataset.get("file_path").is_some());
    assert!(dataset.get("data_memory_bytes").is_some());

    // Verify variables list contains our test variables
    let variables = dataset.get("variables").unwrap().as_array().unwrap();
    let var_names: Vec<_> = variables.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(var_names.contains(&"temperature"));
    assert!(var_names.contains(&"humidity"));
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

#[tokio::test]
async fn test_data_endpoint() {
    // Initialize test environment
    let addr = init_test_environment().await;

    // Test basic query with Arrow format (default) - single variable, single time step
    let response = http_client::get(&addr, "/data?vars=temperature&time_index=0")
        .await
        .expect("Failed to make request");

    // Save the status and content type before we consume the response
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or("unknown"))
        .unwrap_or("missing")
        .to_string(); // Clone the string to avoid borrowing issues

    println!(
        "Data endpoint response: status={}, content-type={}",
        status, content_type
    );

    // Get body as string if status is not 200
    if status != 200 {
        let body = response.text().await.expect("Failed to get response body");
        println!("Error response body: {}", body);
        assert_eq!(
            status, 200,
            "Expected 200 OK, got {} with body: {}",
            status, body
        );
    } else {
        // Convert to bytes for Arrow format checking
        let bytes = response
            .bytes()
            .await
            .expect("Failed to get response bytes");
        assert_eq!(content_type, "application/vnd.apache.arrow.stream");
        assert!(!bytes.is_empty(), "Arrow data should not be empty");
    }

    // Test with specific dimension selections (Arrow format)
    let response = http_client::get(
        &addr,
        "/data?vars=temperature,humidity&time_index=0&lat_range=10,30",
    )
    .await
    .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    // Test with layout specification (Arrow format)
    let response = http_client::get(
        &addr,
        "/data?vars=temperature&time_index=0&layout=latitude,longitude",
    )
    .await
    .expect("Failed to make request");

    assert_eq!(response.status(), 200);

    // Test basic query with JSON format
    let response = http_client::get(&addr, "/data?vars=temperature&time_index=0&format=json")
        .await
        .expect("Failed to make request");

    // Save the status and content type
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or("unknown"))
        .unwrap_or("missing")
        .to_string();

    println!(
        "JSON data endpoint response: status={}, content-type={}",
        status, content_type
    );

    assert_eq!(status, 200);
    assert_eq!(content_type, "application/json");

    // Parse the JSON response and check its structure
    let body = response.text().await.expect("Failed to get response body");
    let json: serde_json::Value = serde_json::from_str(&body).expect("Failed to parse JSON");

    // Check the basic structure of the JSON response
    assert!(
        json.get("metadata").is_some(),
        "JSON response should have a metadata field"
    );
    assert!(
        json.get("data").is_some(),
        "JSON response should have a data field"
    );

    // Check that the data field has our variable
    let data = json.get("data").unwrap();
    assert!(
        data.get("temperature").is_some(),
        "JSON data should contain the temperature variable"
    );

    // Check that the metadata has the expected fields
    let metadata = json.get("metadata").unwrap();
    assert!(
        metadata.get("query").is_some(),
        "Metadata should include query information"
    );
    assert!(
        metadata.get("dimensions").is_some(),
        "Metadata should include dimensions"
    );
    assert!(
        metadata.get("variables").is_some(),
        "Metadata should include variable metadata"
    );

    // Test JSON format with multiple variables
    let response = http_client::get(
        &addr,
        "/data?vars=temperature,humidity&time_index=0&format=json",
    )
    .await
    .expect("Failed to make request");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = response.text().await.expect("Failed to get response body");
    let json: serde_json::Value = serde_json::from_str(&body).expect("Failed to parse JSON");

    // Check that both variables are in the data field
    let data = json.get("data").unwrap();
    assert!(data.get("temperature").is_some());
    assert!(data.get("humidity").is_some());

    // Test invalid format
    let response = http_client::get(&addr, "/data?vars=temperature&format=invalid")
        .await
        .expect("Failed to make request");

    assert_eq!(
        response.status(),
        400,
        "Expected 400 status for invalid format"
    );

    // Test error cases - common for both formats

    // Add debug logging for nonexistent variable test
    println!(
        "Making request to: http://{}{}",
        addr, "/data?vars=nonexistent"
    );

    // Test error case - invalid variable
    let response = http_client::get(&addr, "/data?vars=nonexistent")
        .await
        .expect("Failed to make request");

    let status = response.status();
    println!("Invalid variable test response status: {}", status);

    if status != 400 {
        // Print the response body for debugging
        let body = response.text().await.expect("Failed to get response body");
        println!(
            "Unexpected success response for nonexistent variable: {}",
            body
        );
    }

    assert_eq!(status, 400, "Expected 400 status for nonexistent variable");

    // Test error case - missing required parameter
    let response = http_client::get(&addr, "/data")
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_image_geography_features() {
    // Initialize test environment
    let addr = init_test_environment().await;

    // Test different map projections

    // Eurocentric view
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&center=eurocentric",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 200);

    // Americas view
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&center=americas",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 200);

    // Pacific view
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&center=pacific",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 200);

    // Custom longitude center
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&center=custom:45.0",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 200);

    // Test dateline crossing (without wrap_longitude should fail)
    // Modified to use coordinates in the 0-360 system
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&bbox=350,-30,10,30",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 400); // Should fail without wrap_longitude

    // Test dateline crossing with wrap_longitude=true
    // Modified to use coordinates in the 0-360 system
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=100&height=80&bbox=350,-30,10,30&wrap_longitude=true",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 200); // Should succeed with wrap_longitude

    // Test upsampling/downsampling
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=800&height=600&resampling=bicubic",
    )
    .await
    .expect("Failed to make request");
    assert_eq!(response.status(), 200);

    let bytes = response
        .bytes()
        .await
        .expect("Failed to get response bytes");
    let img = image::load_from_memory(&bytes).expect("Failed to load image from memory");
    assert!(image_utils::assert_image_dimensions(&img, 800, 600).is_ok());
}
