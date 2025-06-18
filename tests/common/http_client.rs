//! HTTP client utilities for testing.
//!
//! This module provides helper functions for making HTTP requests to the rossby server during tests.

use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

/// Default timeout for HTTP requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Create a default test client
pub fn create_test_client() -> Client {
    Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .build()
        .expect("Failed to build test HTTP client")
}

/// Build a URL for a rossby server endpoint
pub fn build_url(addr: &SocketAddr, path: &str) -> Url {
    format!("http://{}{}", addr, path)
        .parse()
        .expect("Failed to parse URL")
}

/// Make a GET request to the rossby server
pub async fn get(addr: &SocketAddr, path: &str) -> Result<Response, Box<dyn Error>> {
    let client = create_test_client();
    let url = build_url(addr, path);
    println!("Making request to: {}", url);
    Ok(client.get(url).send().await?)
}

/// Make a GET request and parse the JSON response
pub async fn get_json<T: DeserializeOwned>(
    addr: &SocketAddr,
    path: &str,
) -> Result<T, Box<dyn Error>> {
    let response = get(addr, path).await?;

    if response.status() != StatusCode::OK {
        return Err(format!(
            "Unexpected status code: {}, body: {:?}",
            response.status(),
            response.text().await
        )
        .into());
    }

    Ok(response.json::<T>().await?)
}

/// Download an image from the rossby server
pub async fn get_image(addr: &SocketAddr, path: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let response = get(addr, path).await?;

    if response.status() != StatusCode::OK {
        return Err(format!(
            "Unexpected status code: {}, body: {:?}",
            response.status(),
            response.text().await
        )
        .into());
    }

    Ok(response.bytes().await?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let addr: SocketAddr = ([127, 0, 0, 1], 8000).into();
        let url = build_url(&addr, "/test");
        assert_eq!(url.as_str(), "http://127.0.0.1:8000/test");
    }

    #[test]
    fn test_create_test_client() {
        // Just verify we can create a client
        let client = create_test_client();
        // Just check that we can create a client
        assert!(client.get("https://example.com").build().is_ok());
    }
}
