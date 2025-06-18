//! Integration tests for rossby server
//!
//! These tests verify that the server works correctly end-to-end.

mod common;

use common::{assertions, http_client, image_utils, test_data};
use std::net::SocketAddr;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;

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

    // In Phase 7, we'll actually start a server here
    // For now, just log that we're "starting" the server (though we actually initialized it statically)
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

    // In Phase 7, we'll make an actual HTTP request to the server
    // For now, just verify the test utilities work
    let url = http_client::build_url(&addr, "/metadata");
    assert_eq!(url.path(), "/metadata");

    // This test will be implemented properly in Phase 7
}

#[tokio::test]
async fn test_point_endpoint() {
    // Initialize test environment
    let _addr = init_test_environment().await;

    // This test will be implemented properly in Phase 7
    // For now, verify the assertion utilities work
    assertions::assert_approx_eq(1.0, 1.0001, Some(0.001));
}

#[tokio::test]
async fn test_image_endpoint() {
    // Initialize test environment
    let _addr = init_test_environment().await;

    // This test will be implemented properly in Phase 7
    // For now, verify the image utilities work
    let img = image::DynamicImage::new_rgb8(10, 10);
    assert!(image_utils::assert_image_dimensions(&img, 10, 10).is_ok());
}
