//! Sequential colormaps (single-hue progression).
//!
//! These colormaps are suitable for data that progresses from low to high.

use super::colormap::Colormap;

/// Viridis colormap - perceptually uniform, colorblind-friendly
pub struct Viridis;

impl Colormap for Viridis {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual viridis colormap
        // This is a placeholder that will be implemented in Phase 6
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }

    fn name(&self) -> &str {
        "viridis"
    }
}

/// Plasma colormap
pub struct Plasma;

impl Colormap for Plasma {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual plasma colormap
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }

    fn name(&self) -> &str {
        "plasma"
    }
}

/// Inferno colormap
pub struct Inferno;

impl Colormap for Inferno {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual inferno colormap
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }

    fn name(&self) -> &str {
        "inferno"
    }
}

/// Magma colormap
pub struct Magma;

impl Colormap for Magma {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual magma colormap
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }

    fn name(&self) -> &str {
        "magma"
    }
}

/// Cividis colormap - colorblind-friendly alternative to viridis
pub struct Cividis;

impl Colormap for Cividis {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // TODO: Implement actual cividis colormap
        let v = (value * 255.0) as u8;
        [v, v, v, 255]
    }

    fn name(&self) -> &str {
        "cividis"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colormap_names() {
        assert_eq!(Viridis.name(), "viridis");
        assert_eq!(Plasma.name(), "plasma");
        assert_eq!(Inferno.name(), "inferno");
        assert_eq!(Magma.name(), "magma");
        assert_eq!(Cividis.name(), "cividis");
    }
}
