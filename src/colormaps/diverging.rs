//! Diverging colormaps (two-hue progression with center).
//!
//! These colormaps are suitable for data that diverges from a central value.

use super::colormap::Colormap;

/// Coolwarm colormap - blue to red through white
pub struct Coolwarm;

impl Colormap for Coolwarm {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Coolwarm colormap data (RGB triplets)
        // Blue to white to red - good for temperature data
        let colors = [
            [59, 76, 192], // Dark blue
            [68, 90, 204],
            [77, 104, 215],
            [87, 117, 225],
            [98, 130, 234],
            [108, 142, 241],
            [119, 154, 247],
            [130, 165, 251],
            [141, 176, 254],
            [152, 185, 255],
            [163, 194, 255],
            [174, 201, 253],
            [184, 208, 249],
            [194, 213, 244],
            [204, 217, 238],
            [213, 219, 230],
            [221, 221, 221], // White/gray in the middle
            [229, 216, 209],
            [236, 211, 197],
            [241, 204, 185],
            [245, 196, 173],
            [247, 187, 160],
            [247, 177, 148],
            [247, 166, 135],
            [244, 154, 123],
            [241, 141, 111],
            [236, 127, 99],
            [229, 112, 88],
            [222, 96, 77],
            [213, 80, 66],
            [203, 62, 56],
            [192, 40, 47], // Dark red
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
        "coolwarm"
    }
}

/// RdBu colormap - red to blue (reversed coolwarm)
pub struct RdBu;

impl Colormap for RdBu {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined RdBu colormap data (RGB triplets)
        // Red to white to blue
        let colors = [
            [192, 40, 47], // Dark red
            [203, 62, 56],
            [213, 80, 66],
            [222, 96, 77],
            [229, 112, 88],
            [236, 127, 99],
            [241, 141, 111],
            [244, 154, 123],
            [247, 166, 135],
            [247, 177, 148],
            [247, 187, 160],
            [245, 196, 173],
            [241, 204, 185],
            [236, 211, 197],
            [229, 216, 209],
            [221, 221, 221], // White/gray in the middle
            [213, 219, 230],
            [204, 217, 238],
            [194, 213, 244],
            [184, 208, 249],
            [174, 201, 253],
            [163, 194, 255],
            [152, 185, 255],
            [141, 176, 254],
            [130, 165, 251],
            [119, 154, 247],
            [108, 142, 241],
            [98, 130, 234],
            [87, 117, 225],
            [77, 104, 215],
            [68, 90, 204],
            [59, 76, 192], // Dark blue
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
        "rdbu"
    }
}

/// Seismic colormap - blue-white-red for anomalies
pub struct Seismic;

impl Colormap for Seismic {
    fn map_normalized(&self, value: f32) -> [u8; 4] {
        // Pre-defined Seismic colormap data (RGB triplets)
        // Blue to white to red - specifically designed for seismic data
        let colors = [
            [0, 0, 127], // Dark blue
            [0, 0, 191],
            [0, 63, 255],
            [0, 127, 255],
            [0, 191, 255],
            [127, 223, 255],
            [191, 239, 255],
            [255, 255, 255], // White in the middle
            [255, 239, 191],
            [255, 223, 127],
            [255, 191, 0],
            [255, 127, 0],
            [255, 63, 0],
            [191, 0, 0],
            [127, 0, 0], // Dark red
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

    #[test]
    fn test_coolwarm_bounds() {
        let colormap = Coolwarm;

        // Test extreme values
        let blue = colormap.map_normalized(0.0);
        let red = colormap.map_normalized(1.0);

        // Coolwarm goes from blue to red
        assert!(blue[2] > blue[0]); // Blue component should be strongest
        assert!(red[0] > red[2]); // Red component should be strongest

        // Test the middle value (should be whitish)
        let middle = colormap.map_normalized(0.5);
        // Middle should be close to white/light gray
        assert!(middle[0] > 200);
        assert!(middle[1] > 200);
        assert!(middle[2] > 200);
    }

    #[test]
    fn test_seismic_middle() {
        let colormap = Seismic;

        // Test the middle value (should be white)
        let middle = colormap.map_normalized(0.5);
        // Should be exactly white
        assert_eq!(middle, [255, 255, 255, 255]);
    }
}
