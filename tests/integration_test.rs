//! Integration tests for rossby server
//!
//! These tests verify that the server works correctly end-to-end.

mod common;

use common::{http_client, image_utils, test_data};
use std::net::SocketAddr;
use std::sync::Once;

// Global test setup for server
use once_cell::sync::OnceCell;

static TEST_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Creates a unique directory for each test server instance.
fn create_unique_temp_dir_and_file(
    generator_fn: fn(&std::path::Path) -> netcdf::Result<()>,
) -> (tempfile::TempDir, String) {
    let count = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_name = format!("test_data_{}.nc", count);
    let file_path = dir.path().join(file_name);
    generator_fn(&file_path).expect("Failed to create test NC file");
    (dir, file_path.to_string_lossy().to_string())
}

/// Start a test server with a specific data generator function.
async fn start_test_server_with_data(
    generator_fn: fn(&std::path::Path) -> netcdf::Result<()>,
) -> SocketAddr {
    let (_temp_dir, file_path) = create_unique_temp_dir_and_file(generator_fn);

    // Use port 0 to let the OS assign an available port
    let addr = SocketAddr::from((std::net::Ipv4Addr::new(127, 0, 0, 1), 0));

    // Start by creating a listener to get the OS-assigned port
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port");

    // Get the actual bound address with the OS-assigned port
    let bound_addr = listener.local_addr().expect("Failed to get local address");

    // Start the server
    tokio::spawn(async move {
        // Create a minimal config
        let config = rossby::Config {
            server: rossby::config::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: bound_addr.port(), // Use the OS-assigned port
                workers: Some(1),
                discovery_url: None,
            },
            ..Default::default()
        };

        // Load the test NetCDF file
        let app_state = rossby::data_loader::load_netcdf(
            std::path::Path::new(&file_path), // Use the generated file path
            config.clone(),
        )
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
            .layer(tower_http::cors::CorsLayer::permissive())
            .with_state(state);

        println!("Test server started on {}", bound_addr);

        axum::serve(listener, app).await.expect("Server error");
    });

    // The server will take some time to start up fully
    println!("Test server starting on {}", bound_addr);

    bound_addr
}

/// Initialize a new test server for each test using the default weather data
async fn init_test_environment() -> SocketAddr {
    init_test_environment_with_data_generator(test_data::create_test_weather_nc).await
}

/// Initialize a new test server with a specific data generator function
async fn init_test_environment_with_data_generator(
    generator_fn: fn(&std::path::Path) -> netcdf::Result<()>,
) -> SocketAddr {
    let server_addr = start_test_server_with_data(generator_fn).await;

    println!(
        "Test server with custom data started, waiting for it to be ready at {}",
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
async fn test_point_variable_validation() {
    let addr = init_test_environment().await; // Uses default test_weather.nc

    // 1. Single valid variable
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&vars=temperature",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 200);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(json.get("temperature").is_some());

    // 2. Multiple valid variables
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&vars=temperature,humidity",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 200);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(json.get("temperature").is_some());
    assert!(json.get("humidity").is_some());

    // 3. Single invalid variable
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&vars=nonexistent_var",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 400);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(
        json["error"],
        "Invalid variable(s): [nonexistent_var]"
    );

    // 4. Multiple variables, one invalid
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&vars=temperature,nonexistent_var",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 400);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(
        json["error"],
        "Invalid variable(s): [nonexistent_var]"
    );

    // 5. Multiple variables, multiple invalid
    let response = http_client::get(
        &addr,
        "/point?lon=-170.0&lat=10.0&vars=temperature,fake1,humidity,fake2",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 400);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    // The order of invalid variables might not be guaranteed, so check for presence
    let error_msg = json["error"].as_str().unwrap();
    assert!(error_msg.starts_with("Invalid variable(s): ["));
    assert!(error_msg.contains("fake1"));
    assert!(error_msg.contains("fake2"));
    assert!(error_msg.ends_with("]"));
}

#[tokio::test]
async fn test_image_variable_validation() {
    let addr = init_test_environment().await; // Uses default test_weather.nc

    // 1. Valid variable
    let response = http_client::get(
        &addr,
        "/image?var=temperature&time_index=0&width=10&height=10",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    // 2. Invalid variable
    let response = http_client::get(
        &addr,
        "/image?var=nonexistent_var&time_index=0&width=10&height=10",
    )
    .await
    .expect("Request failed");
    assert_eq!(response.status(), 400);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(
        json["error"],
        "Invalid variable(s): [nonexistent_var]"
    );
}

#[tokio::test]
async fn test_image_rendering_suitability() {
    let addr =
        init_test_environment_with_data_generator(test_data::create_varied_content_nc).await;

    // 1. Valid 2D spatial variable (temp_map_2d)
    let response = http_client::get(
        &addr,
        "/image?var=temp_map_2d&width=10&height=10", // No time_index needed as it's 2D
    )
    .await
    .expect("Request failed for temp_map_2d");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    // 2. Valid 3D spatial variable (temp_3d_var), will be sliced by time_index=0 default
    let response = http_client::get(
        &addr,
        "/image?var=temp_3d_var&width=10&height=10",
    )
    .await
    .expect("Request failed for temp_3d_var");
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");


    // 3. 1D variable (time_profile_1d) - Not suitable
    let response = http_client::get(
        &addr,
        "/image?var=time_profile_1d&width=10&height=10",
    )
    .await
    .expect("Request failed for time_profile_1d");
    assert_eq!(response.status(), 400);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON for time_profile_1d");
    assert_eq!(
        json["error"],
        "Variable time_profile_1d is not suitable for image rendering. It must be a 2D grid with latitude and longitude dimensions."
    );

    // 4. 2D non-spatial variable (level_data_2d) - Not suitable
    let response = http_client::get(
        &addr,
        "/image?var=level_data_2d&width=10&height=10",
    )
    .await
    .expect("Request failed for level_data_2d");
    assert_eq!(response.status(), 400);
    let json: serde_json::Value = response.json().await.expect("Failed to parse JSON for level_data_2d");
    assert_eq!(
        json["error"],
        "Variable level_data_2d is not suitable for image rendering. It must be a 2D grid with latitude and longitude dimensions."
    );
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
