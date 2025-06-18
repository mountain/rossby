//! Metadata endpoint handler.
//!
//! Returns JSON describing all variables, dimensions, and attributes of the loaded file.

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::state::AppState;

/// Handle GET /metadata requests
pub async fn metadata_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Return the metadata as JSON
    Json(serde_json::json!({
        "global_attributes": state.metadata.global_attributes,
        "dimensions": state.metadata.dimensions,
        "variables": state.metadata.variables,
        "coordinates": state.metadata.coordinates.keys().collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder() {
        // TODO: Add tests when implementation is complete
        assert!(true);
    }
}
