//! Heartbeat endpoint handler.
//!
//! Returns server status information, including uptime, memory usage, and dataset information.

use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, info};
use uuid::Uuid;

use crate::logging::generate_request_id;
use crate::state::AppState;

/// Static server ID generated at compile time
static SERVER_ID: once_cell::sync::Lazy<String> =
    once_cell::sync::Lazy::new(|| Uuid::new_v4().to_string());

/// Server start time
static START_TIME: once_cell::sync::Lazy<SystemTime> = once_cell::sync::Lazy::new(SystemTime::now);

/// Heartbeat response structure
#[derive(Serialize)]
pub struct HeartbeatResponse {
    /// Server ID (unique per instance)
    pub server_id: String,
    /// Current timestamp (ISO 8601 format)
    pub timestamp: String,
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Process memory usage in bytes
    pub memory_usage_bytes: Option<u64>,
    /// Available system memory in bytes
    pub available_memory_bytes: Option<u64>,
    /// Dataset information
    pub dataset: DatasetInfo,
    /// Server status
    pub status: String,
}

/// Dataset information structure
#[derive(Serialize)]
pub struct DatasetInfo {
    /// Dataset file path
    pub file_path: String,
    /// Number of variables
    pub variable_count: usize,
    /// List of variable names
    pub variables: Vec<String>,
    /// Number of dimensions
    pub dimension_count: usize,
    /// Map of dimension names to sizes
    pub dimensions: Vec<(String, usize)>,
    /// Approximate memory usage for dataset in bytes
    pub data_memory_bytes: usize,
}

/// Handle GET /heartbeat requests
pub async fn heartbeat_handler(State(state): State<Arc<AppState>>) -> Json<HeartbeatResponse> {
    let request_id = generate_request_id();
    let start_time = Instant::now();

    debug!(
        endpoint = "/heartbeat",
        request_id = %request_id,
        "Processing heartbeat request"
    );

    // Get current timestamp
    let now = SystemTime::now();
    let timestamp = chrono::DateTime::<chrono::Utc>::from(now)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    // Calculate uptime
    let uptime = now
        .duration_since(*START_TIME)
        .unwrap_or(Duration::from_secs(0));

    debug!(
        uptime_seconds = uptime.as_secs(),
        "Server uptime calculated"
    );

    // Get memory usage (platform-dependent)
    let memory_usage = get_memory_usage();
    let available_memory = get_available_memory();

    // Calculate approximate memory used by the dataset
    let data_memory = calculate_data_memory_usage(&state);

    debug!(
        memory_usage_mb = memory_usage.map(|b| b / (1024 * 1024)),
        available_memory_mb = available_memory.map(|b| b / (1024 * 1024)),
        data_memory_mb = data_memory / (1024 * 1024),
        "Memory statistics collected"
    );

    // Prepare dataset information
    let dataset_info = DatasetInfo {
        file_path: state.config.data.file_path.clone().map_or_else(
            || "<unknown>".to_string(),
            |p| p.to_string_lossy().to_string(),
        ),
        variable_count: state.metadata.variables.len(),
        variables: state.metadata.variables.keys().cloned().collect(),
        dimension_count: state.metadata.dimensions.len(),
        dimensions: state
            .metadata
            .dimensions
            .iter()
            .map(|(name, dim)| (name.clone(), dim.size))
            .collect(),
        data_memory_bytes: data_memory,
    };

    // Create response
    let response = HeartbeatResponse {
        server_id: SERVER_ID.clone(),
        timestamp,
        uptime_seconds: uptime.as_secs(),
        memory_usage_bytes: memory_usage,
        available_memory_bytes: available_memory,
        dataset: dataset_info,
        status: "healthy".to_string(),
    };

    let duration = start_time.elapsed();
    info!(
        endpoint = "/heartbeat",
        request_id = %request_id,
        duration_us = duration.as_micros() as u64,
        uptime_seconds = uptime.as_secs(),
        memory_usage_mb = memory_usage.map(|b| b / (1024 * 1024)),
        data_memory_mb = data_memory / (1024 * 1024),
        variable_count = state.metadata.variables.len(),
        "Heartbeat request successful"
    );

    Json(response)
}

/// Calculate approximate memory usage of the dataset
fn calculate_data_memory_usage(state: &AppState) -> usize {
    let mut total_bytes = 0;

    // Add up the size of each ndarray
    for array in state.data.values() {
        // Each element is a f32 (4 bytes)
        total_bytes += array.len() * 4;
    }

    total_bytes
}

/// Get current process memory usage (platform-dependent)
fn get_memory_usage() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::Read;

        // Read from /proc/self/statm on Linux
        let mut statm = String::new();
        if let Ok(mut file) = File::open("/proc/self/statm") {
            if file.read_to_string(&mut statm).is_ok() {
                let parts: Vec<&str> = statm.split_whitespace().collect();
                if parts.len() >= 2 {
                    // RSS (Resident Set Size) is the second value, in pages
                    if let Ok(pages) = parts[1].parse::<u64>() {
                        // Convert pages to bytes (usually 4KB per page)
                        return Some(pages * 4096);
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Use `ps` command on macOS
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output();

        if let Ok(output) = output {
            let rss = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u64>();
            if let Ok(rss_kb) = rss {
                // Convert KB to bytes
                return Some(rss_kb * 1024);
            }
        }
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Get available system memory (platform-dependent)
fn get_available_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Read from /proc/meminfo on Linux
        if let Ok(file) = File::open("/proc/meminfo") {
            let reader = BufReader::new(file);
            for line in reader.lines().map_while(Result::ok) {
                if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            // Convert KB to bytes
                            return Some(kb * 1024);
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Use `vm_stat` command on macOS
        let output = Command::new("vm_stat").output();

        if let Ok(output) = output {
            let vm_stat = String::from_utf8_lossy(&output.stdout);

            // Parse page size
            let page_size = if let Some(line) = vm_stat.lines().find(|l| l.contains("page size of"))
            {
                if let Some(size_str) = line.split("page size of ").nth(1) {
                    size_str.trim().parse::<u64>().unwrap_or(4096)
                } else {
                    4096 // Default page size (4KB)
                }
            } else {
                4096
            };

            // Find free pages
            if let Some(line) = vm_stat.lines().find(|l| l.starts_with("Pages free:")) {
                if let Some(count_str) = line.split(':').nth(1) {
                    if let Ok(count) = count_str.trim().replace(".", "").parse::<u64>() {
                        return Some(count * page_size);
                    }
                }
            }
        }
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::Metadata;
    use std::collections::HashMap;

    #[test]
    fn test_heartbeat_response_structure() {
        // Create a minimal AppState for testing
        let config = Config::default();

        // Create dimensions with proper structure
        let mut dimensions = HashMap::new();
        dimensions.insert(
            "lat".to_string(),
            crate::state::Dimension {
                name: "lat".to_string(),
                size: 180,
                is_unlimited: false,
            },
        );
        dimensions.insert(
            "lon".to_string(),
            crate::state::Dimension {
                name: "lon".to_string(),
                size: 360,
                is_unlimited: false,
            },
        );
        dimensions.insert(
            "time".to_string(),
            crate::state::Dimension {
                name: "time".to_string(),
                size: 24,
                is_unlimited: true,
            },
        );

        let metadata = Metadata {
            dimensions,
            variables: HashMap::new(),
            global_attributes: HashMap::new(),
            coordinates: HashMap::new(),
        };

        let data = HashMap::new();

        let state = AppState::new(config, metadata, data);

        // Calculate data memory usage
        let memory = calculate_data_memory_usage(&state);

        // Since we have no data, it should be 0
        assert_eq!(memory, 0);
    }
}
