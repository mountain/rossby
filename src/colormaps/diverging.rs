//! Diverging colormaps (two-hue progression with center).
//!
//! These colormaps are suitable for data that diverges from a central value.

use super::colormap::Colormap;

/// Coolwarm colormap - blue to red through white
pub struct Coolwarm;

impl Colormap for Coolwarm {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual coolwarm colormap
        // This is a placeholder that will be implemented in Phase 6
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }
    
    fn name(&self) -> &str {
        "coolwarm"
    }
}

/// RdBu colormap - red to blue (reversed coolwarm)
pub struct RdBu;

impl Colormap for RdBu {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual RdBu colormap
        // This is a placeholder that will be implemented in Phase 6
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }
    
    fn name(&self) -> &str {
        "rdbu"
    }
}

/// Seismic colormap - blue-white-red for anomalies
pub struct Seismic;

impl Colormap for Seismic {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual seismic colormap
        // This is a placeholder that will be implemented in Phase 6
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }
    
    fn name(&self) -> &str {
        "seismic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colormap_names() {
        assert_eq!(Coolwarm.name(), "coolwarm");
        assert_eq!(RdBu.name(), "rdbu");
        assert_eq!(Seismic.name(), "seismic");
    }
}
>>>>>>> REPLACE
