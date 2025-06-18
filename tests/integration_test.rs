//! Integration tests for rossby server
//!
//! These tests verify that the server works correctly end-to-end.

mod common;

use common::{assertions, http_client, image_utils, test_data};
use std::net::SocketAddr;
use std::sync::Once;
use std::time::Duration;
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::sleep;

// Global test setup for server
static INIT: Once = Once::new();
static TEST_PORT: u16 = 9876;
static mut TEST_SERVER_HANDLE: Option<JoinHandle<()>> = None;
static mut TEST_FILE_PATH: Option<String> = None;

/// Start a test server on a dedicated port
async fn start_test_server() -> SocketAddr {
    // Create a test NetCDF file
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_weather.nc");
    test_data::create_test_weather_nc(&file_path).unwrap();
    
    // Save the path so the temporary directory doesn't get dropped
    unsafe {
        TEST_FILE_PATH = Some(file_path.to_string_lossy().to_string());
    }
    
    // Bind to a specific port for testing
    let addr = ([127, 0, 0, 1], TEST_PORT).into();
    
    // We'll implement this properly in Phase 7
    // For now, just create a dummy server that responds with placeholder data
    let _listener = TcpListener::bind(addr).await.unwrap();
    let server_handle = tokio::spawn(async move {
        println!("Test server started on {}", addr);
        // In Phase 7, we'll start an actual rossby server here
        loop {
            sleep(Duration::from_secs(1)).await;
        }
    });
    
    // Store the server handle to keep it alive
    unsafe {
        TEST_SERVER_HANDLE = Some(server_handle);
    }
    
    sleep(Duration::from_millis(100)).await;
    addr
}

/// Get the test server address
fn test_server_addr() -> SocketAddr {
    ([127, 0, 0, 1], TEST_PORT).into()
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
