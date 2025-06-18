//! Colormap implementations for image generation.
//!
//! This module provides matplotlib-inspired colormaps for visualizing data.

pub mod colormap;
pub mod diverging;
pub mod sequential;

pub use colormap::{get_colormap, Colormap};

// Re-export commonly used colormaps
pub use diverging::{Coolwarm, RdBu, Seismic};
pub use sequential::{Cividis, Inferno, Magma, Plasma, Viridis};
