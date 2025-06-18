//! Sequential colormaps (single-hue progression).
//!
//! These colormaps are suitable for data that progresses from low to high.

use super::colormap::Colormap;

/// Viridis colormap - perceptually uniform, colorblind-friendly
pub struct Viridis;

impl Colormap for Viridis {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Viridis colormap data (RGB triplets)
        // These are sampled from the matplotlib Viridis colormap
        let colors = [
            [68, 1, 84], // Dark purple
            [72, 33, 115],
            [64, 67, 135],
            [52, 94, 141],
            [41, 120, 142],
            [32, 144, 140],
            [34, 167, 132],
            [68, 190, 112],
            [121, 209, 81],
            [189, 222, 38],
            [253, 231, 36], // Yellow
        ];

        // Calculate the position in our color array
        let position = value * (colors.len() - 1) as f32;
        let index = position.floor() as usize;

        // Get the two colors to interpolate between
        if index >= colors.len() - 1 {
            // Last color
            return [
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                255,
            ];
        }

        let t = position - index as f32; // Fractional part for interpolation
        let c1 = colors[index];
        let c2 = colors[index + 1];

        // Interpolate between the two colors
        let rgb = super::colormap::lerp_color(c1, c2, t);
        [rgb[0], rgb[1], rgb[2], 255]
    }

    fn name(&self) -> &str {
        "viridis"
    }
}

/// Plasma colormap
pub struct Plasma;

impl Colormap for Plasma {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Plasma colormap data (RGB triplets)
        let colors = [
            [13, 8, 135], // Dark blue
            [75, 3, 161],
            [125, 3, 168],
            [168, 13, 155],
            [203, 30, 129],
            [228, 55, 97],
            [246, 82, 66],
            [251, 118, 35],
            [246, 157, 8],
            [232, 197, 0],
            [240, 249, 33], // Yellow
        ];

        // Calculate the position in our color array
        let position = value * (colors.len() - 1) as f32;
        let index = position.floor() as usize;

        // Get the two colors to interpolate between
        if index >= colors.len() - 1 {
            // Last color
            return [
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                255,
            ];
        }

        let t = position - index as f32; // Fractional part for interpolation
        let c1 = colors[index];
        let c2 = colors[index + 1];

        // Interpolate between the two colors
        let rgb = super::colormap::lerp_color(c1, c2, t);
        [rgb[0], rgb[1], rgb[2], 255]
    }

    fn name(&self) -> &str {
        "plasma"
    }
}

/// Inferno colormap
pub struct Inferno;

impl Colormap for Inferno {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Inferno colormap data (RGB triplets)
        let colors = [
            [0, 0, 4], // Black
            [22, 11, 57],
            [66, 10, 104],
            [106, 23, 110],
            [147, 38, 103],
            [186, 54, 85],
            [221, 73, 64],
            [243, 106, 39],
            [251, 150, 24],
            [246, 196, 40],
            [252, 255, 164], // Light yellow
        ];

        // Calculate the position in our color array
        let position = value * (colors.len() - 1) as f32;
        let index = position.floor() as usize;

        // Get the two colors to interpolate between
        if index >= colors.len() - 1 {
            // Last color
            return [
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                255,
            ];
        }

        let t = position - index as f32; // Fractional part for interpolation
        let c1 = colors[index];
        let c2 = colors[index + 1];

        // Interpolate between the two colors
        let rgb = super::colormap::lerp_color(c1, c2, t);
        [rgb[0], rgb[1], rgb[2], 255]
    }

    fn name(&self) -> &str {
        "inferno"
    }
}

/// Magma colormap
pub struct Magma;

impl Colormap for Magma {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Magma colormap data (RGB triplets)
        let colors = [
            [0, 0, 4], // Black
            [28, 16, 68],
            [79, 18, 123],
            [129, 37, 129],
            [181, 54, 122],
            [229, 80, 100],
            [251, 135, 97],
            [254, 194, 135],
            [252, 253, 191], // Light yellow
        ];

        // Calculate the position in our color array
        let position = value * (colors.len() - 1) as f32;
        let index = position.floor() as usize;

        // Get the two colors to interpolate between
        if index >= colors.len() - 1 {
            // Last color
            return [
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                255,
            ];
        }

        let t = position - index as f32; // Fractional part for interpolation
        let c1 = colors[index];
        let c2 = colors[index + 1];

        // Interpolate between the two colors
        let rgb = super::colormap::lerp_color(c1, c2, t);
        [rgb[0], rgb[1], rgb[2], 255]
    }

    fn name(&self) -> &str {
        "magma"
    }
}

/// Cividis colormap - colorblind-friendly alternative to viridis
pub struct Cividis;

impl Colormap for Cividis {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Cividis colormap data (RGB triplets)
        // Designed to be perceptually uniform and colorblind-friendly
        let colors = [
            [0, 32, 76], // Dark blue
            [0, 42, 102],
            [0, 52, 110],
            [25, 63, 115],
            [46, 73, 114],
            [67, 84, 107],
            [90, 96, 98],
            [115, 106, 85],
            [143, 117, 70],
            [172, 129, 54],
            [206, 142, 36],
            [237, 158, 16],
            [255, 172, 0], // Yellow
        ];

        // Calculate the position in our color array
        let position = value * (colors.len() - 1) as f32;
        let index = position.floor() as usize;

        // Get the two colors to interpolate between
        if index >= colors.len() - 1 {
            // Last color
            return [
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                255,
            ];
        }

        let t = position - index as f32; // Fractional part for interpolation
        let c1 = colors[index];
        let c2 = colors[index + 1];

        // Interpolate between the two colors
        let rgb = super::colormap::lerp_color(c1, c2, t);
        [rgb[0], rgb[1], rgb[2], 255]
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

    #[test]
    fn test_viridis_bounds() {
        let colormap = Viridis;

        // Test extreme values
        let black = colormap.map_normalized(0.0);
        let yellow = colormap.map_normalized(1.0);

        // Viridis goes from dark purple to yellow
        assert_eq!(black, [68, 1, 84, 255]);
        assert_eq!(yellow, [253, 231, 36, 255]);

        // Test an intermediate value
        let middle = colormap.map_normalized(0.5);
        // Middle should be greenish
        assert!(middle[1] > middle[0]); // Green component should be strongest
        assert!(middle[1] > middle[2]);
    }
}
