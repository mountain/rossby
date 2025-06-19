//! Colormap implementations for image generation.
//!
//! This module provides matplotlib-inspired colormaps for visualizing data
//! and geographic utilities for visualization.

pub mod colormap;
pub mod diverging;
pub mod geoutil;
pub mod sequential;

pub use colormap::{get_colormap, Colormap};

// Re-export commonly used colormaps
pub use diverging::{Coolwarm, RdBu, Seismic};
pub use sequential::{Cividis, Inferno, Magma, Plasma, Viridis};

// Re-export geography utilities
pub use geoutil::{
    adjust_for_dateline_crossing, handle_dateline_crossing_bbox, normalize_longitude, parse_bbox,
    resample_data, MapProjection,
};
