//! Logging utilities for the rossby server.
//!
//! This module provides structured logging functionality to make logs more
//! searchable, analyzable, and useful for production deployments.

use std::time::Instant;
use tracing::{debug, error, info, warn, Level};

use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_http::{trace::OnResponse, LatencyUnit};
use uuid::Uuid;

/// Creates the tracing layer for HTTP request/response logging
pub fn create_http_trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    impl OnResponse<tower_http::classify::ServerErrorsFailureClass>,
> {
    // Create a custom response formatter that includes timing
    let response_formatter = DefaultOnResponse::new()
        .level(Level::DEBUG)
        .latency_unit(LatencyUnit::Micros);

    // Configure the tracing layer
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true),
        )
        .on_request(DefaultOnRequest::new().level(Level::DEBUG))
        .on_response(response_formatter)
}

/// Initialize the tracing subscriber with the given log level
pub fn init_tracing(log_level: &str) {
    let filter = match std::env::var("RUST_LOG") {
        Ok(val) => val,
        Err(_) => log_level.to_string(),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

/// Log a start message for a significant operation
pub fn log_operation_start(operation: &str, details: Option<&str>) {
    if let Some(details) = details {
        info!(
            operation = operation,
            details = details,
            "Starting operation"
        );
    } else {
        info!(operation = operation, "Starting operation");
    }
}

/// Log the completion of a significant operation
pub fn log_operation_end(operation: &str, start_time: Instant, success: bool) {
    let duration = start_time.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;

    if success {
        info!(
            operation = operation,
            duration_ms = duration_ms,
            "Operation completed successfully"
        );
    } else {
        warn!(
            operation = operation,
            duration_ms = duration_ms,
            "Operation completed with warnings"
        );
    }
}

/// Log an operation with timing and result in a single statement
pub fn log_timed_operation<F, R>(operation: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let request_id = Uuid::new_v4();

    debug!(
        operation = operation,
        request_id = %request_id,
        "Starting operation"
    );

    let result = f();

    let duration = start.elapsed();

    info!(
        operation = operation,
        request_id = %request_id,
        duration_ms = duration.as_secs_f64() * 1000.0,
        "Operation completed"
    );

    result
}

/// Log detailed information about the data loaded
pub fn log_data_load_stats(
    file_path: &str,
    var_count: usize,
    var_names: &[&str],
    dim_count: usize,
    dim_details: &str,
    memory_usage: usize,
) {
    info!(
        operation = "data_load",
        file_path = file_path,
        var_count = var_count,
        vars = %var_names.join(", "),
        dim_count = dim_count,
        dims = dim_details,
        memory_mb = memory_usage / (1024 * 1024),
        "Data loaded successfully"
    );
}

/// Log an error with context
pub fn log_error(error: &crate::error::RossbyError, context: &str) {
    error!(
        error = %error,
        context = context,
        error_type = std::any::type_name_of_val(error),
        "Error occurred"
    );
}

/// Log an error that occurred during request processing
pub fn log_request_error(
    error: &crate::error::RossbyError,
    endpoint: &str,
    request_id: &str,
    params: Option<&str>,
) {
    error!(
        error = %error,
        endpoint = endpoint,
        request_id = request_id,
        params = params.unwrap_or("none"),
        error_type = std::any::type_name_of_val(error),
        "Request processing error"
    );
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();

        assert!(!id1.is_empty());
        assert_ne!(id1, id2); // IDs should be unique
    }

    #[test]
    fn test_log_timed_operation() {
        // This is more of a functional test to ensure it doesn't panic
        let result = log_timed_operation("test_operation", || {
            // Simulate some work
            std::thread::sleep(Duration::from_millis(1));
            42
        });

        assert_eq!(result, 42);
    }
}
