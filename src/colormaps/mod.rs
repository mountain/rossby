//! Colormap implementations for image generation.
//!
//! This module provides matplotlib-inspired colormaps for visualizing data.

pub mod colormap;
pub mod sequential;
pub mod diverging;

pub use colormap::{Colormap, get_colormap};

// Re-export commonly used colormaps
pub use sequential::{Viridis, Plasma, Inferno, Magma, Cividis};
pub use diverging::{Coolwarm, RdBu, Seismic};
