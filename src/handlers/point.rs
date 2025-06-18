//! Point query endpoint handler.
//!
//! Returns interpolated values for one or more variables at a specific point in space-time.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::RossbyError;
use crate::state::AppState;

/// Query parameters for point endpoint
#[derive(Debug, Deserialize)]
pub struct PointQuery {
    /// Longitude coordinate
    pub lon: f64,
    /// Latitude coordinate
    pub lat: f64,
    /// Time index (0-based)
    pub time_index: Option<usize>,
    /// Comma-separated list of variables to query
    pub vars: String,
    /// Interpolation method (nearest, bilinear, bicubic)
    pub interpolation: Option<String>,
}

/// Response for point query
#[derive(Debug, Serialize)]
pub struct PointResponse {
    #[serde(flatten)]
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// Handle GET /point requests
pub async fn point_handler(
    State(_state): State<Arc<AppState>>,
    Query(_params): Query<PointQuery>,
) -> Response {
    // TODO: Implement point query with interpolation
    // This is a placeholder that will be implemented in Phase 5

    // For now, return a simple error
    let error = RossbyError::InvalidParameter {
        param: "interpolation".to_string(),
        message: "Point queries not yet implemented".to_string(),
    };

    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": error.to_string()
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_query_parsing() {
        // TODO: Add tests when implementation is complete
        assert!(true);
    }
}
