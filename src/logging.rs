//! Logging utilities for rossby.
//!
//! This module provides structured logging functions for server operations,
//! requests, and errors using the tracing crate.

use std::time::{Duration, Instant};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::error::RossbyError;

/// Generate a unique request ID for tracing
pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// Structured logging for operation timing
///
/// # Examples
///
/// ```
/// use rossby::logging;
///
/// let result: Result<_, ()> = logging::log_timed_operation("data_loading", || {
///     // Operation to time
///     Ok::<_, ()>(42)
/// });
/// assert_eq!(result.unwrap(), 42);
/// ```
pub fn log_timed_operation<F, T, E>(operation: &str, f: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    info!(operation = %operation, "Starting operation");
    let start = Instant::now();

    match f() {
        Ok(result) => {
            let duration = start.elapsed();
            info!(
                operation = %operation,
                duration_ms = duration.as_millis() as u64,
                "Operation completed successfully"
            );
            Ok(result)
        }
        Err(err) => {
            let duration = start.elapsed();
            error!(
                operation = %operation,
                duration_ms = duration.as_millis() as u64,
                "Operation failed"
            );
            Err(err)
        }
    }
}

/// Start timing an operation and return a guard that will log when dropped
pub fn start_timed_operation(operation: &str, details: Option<&str>) -> TimedOperationGuard {
    let operation_str = operation.to_string();

    if let Some(details) = details {
        info!(operation = %operation, details = %details, "Starting operation");
    } else {
        info!(operation = %operation, "Starting operation");
    }

    TimedOperationGuard {
        operation: operation_str,
        start: Instant::now(),
    }
}

/// Guard that logs the duration of an operation when dropped
pub struct TimedOperationGuard {
    operation: String,
    start: Instant,
}

impl Drop for TimedOperationGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        info!(
            operation = %self.operation,
            duration_ms = duration.as_millis() as u64,
            "Operation completed successfully"
        );
    }
}

/// Log successful data loading operation with detailed metrics
pub fn log_data_loaded(
    file_path: &str,
    var_count: usize,
    vars: &[String],
    dim_count: usize,
    dims: &[(String, usize)],
    memory_mb: usize,
) {
    // Format dimensions as "dim1=size1, dim2=size2, ..."
    let dims_str = dims
        .iter()
        .map(|(name, size)| format!("{}={}", name, size))
        .collect::<Vec<_>>()
        .join(", ");

    // Format variables as comma-separated list
    let vars_str = vars.join(", ");

    info!(
        operation = "data_load",
        file_path = %file_path,
        var_count = var_count,
        vars = %vars_str,
        dim_count = dim_count,
        dims = %dims_str,
        memory_mb = memory_mb,
        "Data loaded successfully"
    );
}

/// Log request errors with context
pub fn log_request_error(
    error: &RossbyError,
    endpoint: &str,
    request_id: &str,
    details: Option<&str>,
) {
    let error_type = std::any::type_name::<RossbyError>();

    if let Some(details) = details {
        error!(
            endpoint = %endpoint,
            request_id = %request_id,
            error = %error,
            context = %details,
            error_type = %error_type,
            "Error occurred"
        );
    } else {
        error!(
            endpoint = %endpoint,
            request_id = %request_id,
            error = %error,
            error_type = %error_type,
            "Error occurred"
        );
    }
}

/// Log basic request information
pub fn log_request(
    endpoint: &str,
    request_id: &str,
    method: &str,
    path: &str,
    query: Option<&str>,
) {
    if let Some(query) = query {
        debug!(
            endpoint = %endpoint,
            request_id = %request_id,
            method = %method,
            path = %path,
            query = %query,
            "Request received"
        );
    } else {
        debug!(
            endpoint = %endpoint,
            request_id = %request_id,
            method = %method,
            path = %path,
            "Request received"
        );
    }
}

/// Log successful request completion
pub fn log_request_success(endpoint: &str, request_id: &str, status: u16, duration: Duration) {
    info!(
        endpoint = %endpoint,
        request_id = %request_id,
        status = status,
        duration_ms = duration.as_millis() as u64,
        "Request completed successfully"
    );
}

/// Set up logging with appropriate formatting and level
pub fn setup_logging() -> Result<(), RossbyError> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    // Use RUST_LOG env var if set, otherwise use info level
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Initialize the tracing subscriber
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(env_filter)
        .try_init()
        .map_err(|e| RossbyError::Server {
            message: format!("Failed to initialize logging: {}", e),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        // IDs should be valid UUIDs
        assert_eq!(id1.len(), 36);
        assert_eq!(id2.len(), 36);

        // IDs should be unique
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_log_timed_operation() {
        // This test just verifies that the function works without panicking
        let result = log_timed_operation("test_operation", || Ok::<_, ()>(42));

        assert_eq!(result.unwrap(), 42);

        let error_result: Result<(), &str> =
            log_timed_operation("failing_operation", || Err("test error"));

        assert_eq!(error_result.unwrap_err(), "test error");
    }
}
