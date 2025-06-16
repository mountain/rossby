//! Colormap trait and utilities.
//!
//! This module defines the common interface for all colormaps.

use crate::error::{Result, RossbyError};

/// Trait for color mapping implementations
pub trait Colormap: Send + Sync {
    /// Map a normalized value (0.0 to 1.0) to an RGBA color
    fn map_normalized(&self, value: f32) -> [u8; 4];
    
    /// Map a value to an RGBA color given the data range
    fn map(&self, value: f32, min: f32, max: f32) -> [u8; 4] {
        let normalized = if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        self.map_normalized(normalized)
    }
    
    /// Get the name of this colormap
    fn name(&self) -> &str;
}

/// Get a colormap by name
pub fn get_colormap(name: &str) -> Result<Box<dyn Colormap>> {
    use super::{sequential::*, diverging::*};
    
    match name.to_lowercase().as_str() {
        "viridis" => Ok(Box::new(Viridis)),
        "plasma" => Ok(Box::new(Plasma)),
        "inferno" => Ok(Box::new(Inferno)),
        "magma" => Ok(Box::new(Magma)),
        "cividis" => Ok(Box::new(Cividis)),
        "coolwarm" => Ok(Box::new(Coolwarm)),
        "rdbu" => Ok(Box::new(RdBu)),
        "seismic" => Ok(Box::new(Seismic)),
        _ => Err(RossbyError::InvalidParameter {
            param: "colormap".to_string(),
            message: format!("Unknown colormap: {}", name),
        }),
    }
}

/// Linear interpolation between two colors
pub fn lerp_color(c1: [u8; 3], c2: [u8; 3], t: f32) -> [u8; 3] {
    [
        (c1[0] as f32 * (1.0 - t) + c2[0] as f32 * t) as u8,
        (c1[1] as f32 * (1.0 - t) + c2[1] as f32 * t) as u8,
        (c1[2] as f32 * (1.0 - t) + c2[2] as f32 * t) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp_color() {
        let black = [0, 0, 0];
        let white = [255, 255, 255];
        
        let mid = lerp_color(black, white, 0.5);
        assert_eq!(mid[0], 127);
        assert_eq!(mid[1], 127);
        assert_eq!(mid[2], 127);
    }
}
