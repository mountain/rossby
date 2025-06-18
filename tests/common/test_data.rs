//! Test data generation utilities.
//!
//! This module provides functions to generate various NetCDF test files
//! with known data patterns for testing the rossby server.

use std::path::Path;
use std::io::Result;

/// Creates a NetCDF file with a simple linear gradient pattern.
///
/// # Arguments
///
/// * `path` - The path where the NetCDF file will be saved
/// * `size` - The dimensions of the grid (width, height)
///
/// # Returns
///
/// * `Result<()>` - Ok if successful, or an IO error
pub fn create_linear_gradient_nc(path: &Path, _size: (usize, usize)) -> Result<()> {
    // TODO: Implement actual NetCDF file creation in Phase 4
    // This is a placeholder that will be implemented when the netcdf dependency is restored
    
    // For now, just create an empty file to simulate the test data
    std::fs::write(path, b"PLACEHOLDER_LINEAR_GRADIENT")?;
    
    Ok(())
}

/// Creates a NetCDF file with a sinusoidal pattern.
///
/// # Arguments
///
/// * `path` - The path where the NetCDF file will be saved
/// * `size` - The dimensions of the grid (width, height)
///
/// # Returns
///
/// * `Result<()>` - Ok if successful, or an IO error
pub fn create_sinusoidal_nc(path: &Path, _size: (usize, usize)) -> Result<()> {
    // TODO: Implement actual NetCDF file creation in Phase 4
    // This is a placeholder that will be implemented when the netcdf dependency is restored
    
    // For now, just create an empty file to simulate the test data
    std::fs::write(path, b"PLACEHOLDER_SINUSOIDAL")?;
    
    Ok(())
}

/// Creates a NetCDF file with a gaussian blob pattern.
///
/// # Arguments
///
/// * `path` - The path where the NetCDF file will be saved
/// * `size` - The dimensions of the grid (width, height)
///
/// # Returns
///
/// * `Result<()>` - Ok if successful, or an IO error
pub fn create_gaussian_blob_nc(path: &Path, _size: (usize, usize)) -> Result<()> {
    // TODO: Implement actual NetCDF file creation in Phase 4
    // This is a placeholder that will be implemented when the netcdf dependency is restored
    
    // For now, just create an empty file to simulate the test data
    std::fs::write(path, b"PLACEHOLDER_GAUSSIAN_BLOB")?;
    
    Ok(())
}

/// Creates a NetCDF file with realistic weather data for testing.
///
/// This generates a small but realistic weather dataset with common variables
/// like temperature, wind, pressure, etc. on a geographic grid.
///
/// # Arguments
///
/// * `path` - The path where the NetCDF file will be saved
///
/// # Returns
///
/// * `Result<()>` - Ok if successful, or an IO error
pub fn create_test_weather_nc(path: &Path) -> Result<()> {
    // TODO: Implement actual NetCDF file creation in Phase 4
    // This is a placeholder that will be implemented when the netcdf dependency is restored
    
    // For now, just create an empty file to simulate the test data
    std::fs::write(path, b"PLACEHOLDER_WEATHER_DATA")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_linear_gradient_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("linear_gradient.nc");
        
        assert!(create_linear_gradient_nc(&file_path, (10, 10)).is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_create_sinusoidal_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("sinusoidal.nc");
        
        assert!(create_sinusoidal_nc(&file_path, (10, 10)).is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_create_gaussian_blob_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("gaussian_blob.nc");
        
        assert!(create_gaussian_blob_nc(&file_path, (10, 10)).is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn test_create_test_weather_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("weather_test.nc");
        
        assert!(create_test_weather_nc(&file_path).is_ok());
        assert!(file_path.exists());
    }
}
