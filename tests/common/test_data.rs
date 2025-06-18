//! Test data generation utilities.
//!
//! This module provides functions to generate various NetCDF test files
//! with known data patterns for testing the rossby server.

use netcdf::types::BasicType;
use std::f32::consts::PI;
use std::path::Path;

// We use Result<()> from our crate to handle netcdf-specific errors
use crate::error::Result;

/// Creates a NetCDF file with a simple linear gradient pattern.
///
/// # Arguments
///
/// * `path` - The path where the NetCDF file will be saved
/// * `size` - The dimensions of the grid (width, height)
///
/// # Returns
///
/// * `Result<()>` - Ok if successful, or an error
pub fn create_linear_gradient_nc(path: &Path, size: (usize, usize)) -> Result<()> {
    // Create a new NetCDF file
    let mut file = netcdf::create(path)?;

    // Add dimensions
    let lon_dim = file.add_dimension("lon", size.0)?;
    let lat_dim = file.add_dimension("lat", size.1)?;
    let time_dim = file.add_unlimited_dimension("time")?;

    // Add coordinate variables
    let mut lon_var = file.add_variable::<f32>("lon", &[&lon_dim])?;
    let mut lat_var = file.add_variable::<f32>("lat", &[&lat_dim])?;
    let mut time_var = file.add_variable::<f32>("time", &[&time_dim])?;

    // Add a data variable
    let mut data_var = file.add_variable::<f32>("gradient", &[&time_dim, &lat_dim, &lon_dim])?;

    // Add some attributes
    file.add_attribute("title", "Linear Gradient Test Data")?;
    file.add_attribute("institution", "rossby test suite")?;

    lon_var.add_attribute("units", "degrees_east")?;
    lat_var.add_attribute("units", "degrees_north")?;
    time_var.add_attribute("units", "days since 2000-01-01")?;

    data_var.add_attribute("units", "arbitrary")?;
    data_var.add_attribute("long_name", "Linear Gradient")?;

    // Create coordinate values
    let lon_values: Vec<f32> = (0..size.0)
        .map(|i| (i as f32) * 360.0 / (size.0 as f32))
        .collect();
    let lat_values: Vec<f32> = (0..size.1)
        .map(|i| -90.0 + (i as f32) * 180.0 / (size.1 as f32))
        .collect();
    let time_values: Vec<f32> = vec![0.0, 1.0, 2.0]; // 3 time steps

    // Write coordinate values
    lon_var.put_values(&lon_values, None, None)?;
    lat_var.put_values(&lat_values, None, None)?;
    time_var.put_values(&time_values, None, None)?;

    // Create gradient data
    let total_size = 3 * size.1 * size.0; // 3 time steps
    let mut data_values = Vec::with_capacity(total_size);

    // Generate a linear gradient for each time step
    for t in 0..3 {
        for y in 0..size.1 {
            for x in 0..size.0 {
                // Simple linear gradient from bottom-left to top-right
                let normalized_x = x as f32 / (size.0 - 1) as f32;
                let normalized_y = y as f32 / (size.1 - 1) as f32;
                let value = (normalized_x + normalized_y) / 2.0 * (1.0 + t as f32 * 0.2);
                data_values.push(value);
            }
        }
    }

    // Write the data
    data_var.put_values(&data_values, None, None)?;

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
/// * `Result<()>` - Ok if successful, or an error
pub fn create_sinusoidal_nc(path: &Path, size: (usize, usize)) -> Result<()> {
    // Create a new NetCDF file
    let mut file = netcdf::create(path)?;

    // Add dimensions
    let lon_dim = file.add_dimension("lon", size.0)?;
    let lat_dim = file.add_dimension("lat", size.1)?;
    let time_dim = file.add_unlimited_dimension("time")?;

    // Add coordinate variables
    let mut lon_var = file.add_variable::<f32>("lon", &[&lon_dim])?;
    let mut lat_var = file.add_variable::<f32>("lat", &[&lat_dim])?;
    let mut time_var = file.add_variable::<f32>("time", &[&time_dim])?;

    // Add a data variable
    let mut data_var = file.add_variable::<f32>("wave", &[&time_dim, &lat_dim, &lon_dim])?;

    // Add some attributes
    file.add_attribute("title", "Sinusoidal Pattern Test Data")?;
    file.add_attribute("institution", "rossby test suite")?;

    lon_var.add_attribute("units", "degrees_east")?;
    lat_var.add_attribute("units", "degrees_north")?;
    time_var.add_attribute("units", "days since 2000-01-01")?;

    data_var.add_attribute("units", "arbitrary")?;
    data_var.add_attribute("long_name", "Sinusoidal Wave Pattern")?;

    // Create coordinate values
    let lon_values: Vec<f32> = (0..size.0)
        .map(|i| (i as f32) * 360.0 / (size.0 as f32))
        .collect();
    let lat_values: Vec<f32> = (0..size.1)
        .map(|i| -90.0 + (i as f32) * 180.0 / (size.1 as f32))
        .collect();
    let time_values: Vec<f32> = vec![0.0, 1.0, 2.0]; // 3 time steps

    // Write coordinate values
    lon_var.put_values(&lon_values, None, None)?;
    lat_var.put_values(&lat_values, None, None)?;
    time_var.put_values(&time_values, None, None)?;

    // Create sinusoidal pattern data
    let total_size = 3 * size.1 * size.0; // 3 time steps
    let mut data_values = Vec::with_capacity(total_size);

    // Generate a sinusoidal pattern for each time step
    for t in 0..3 {
        for y in 0..size.1 {
            for x in 0..size.0 {
                // Create a sinusoidal pattern
                let x_normalized = x as f32 / size.0 as f32 * 4.0 * PI;
                let y_normalized = y as f32 / size.1 as f32 * 4.0 * PI;

                // Sin wave in x direction, cos wave in y direction
                let wave_x = (x_normalized).sin();
                let wave_y = (y_normalized).cos();

                // Combine waves and scale by time step
                let value = (wave_x + wave_y) / 2.0 * (1.0 + t as f32 * 0.3);
                data_values.push(value);
            }
        }
    }

    // Write the data
    data_var.put_values(&data_values, None, None)?;

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
/// * `Result<()>` - Ok if successful, or an error
pub fn create_gaussian_blob_nc(path: &Path, size: (usize, usize)) -> Result<()> {
    // Create a new NetCDF file
    let mut file = netcdf::create(path)?;

    // Add dimensions
    let lon_dim = file.add_dimension("lon", size.0)?;
    let lat_dim = file.add_dimension("lat", size.1)?;
    let time_dim = file.add_unlimited_dimension("time")?;

    // Add coordinate variables
    let mut lon_var = file.add_variable::<f32>("lon", &[&lon_dim])?;
    let mut lat_var = file.add_variable::<f32>("lat", &[&lat_dim])?;
    let mut time_var = file.add_variable::<f32>("time", &[&time_dim])?;

    // Add a data variable
    let mut data_var = file.add_variable::<f32>("blob", &[&time_dim, &lat_dim, &lon_dim])?;

    // Add some attributes
    file.add_attribute("title", "Gaussian Blob Test Data")?;
    file.add_attribute("institution", "rossby test suite")?;

    lon_var.add_attribute("units", "degrees_east")?;
    lat_var.add_attribute("units", "degrees_north")?;
    time_var.add_attribute("units", "days since 2000-01-01")?;

    data_var.add_attribute("units", "arbitrary")?;
    data_var.add_attribute("long_name", "Gaussian Blob Pattern")?;

    // Create coordinate values
    let lon_values: Vec<f32> = (0..size.0)
        .map(|i| (i as f32) * 360.0 / (size.0 as f32))
        .collect();
    let lat_values: Vec<f32> = (0..size.1)
        .map(|i| -90.0 + (i as f32) * 180.0 / (size.1 as f32))
        .collect();
    let time_values: Vec<f32> = vec![0.0, 1.0, 2.0]; // 3 time steps

    // Write coordinate values
    lon_var.put_values(&lon_values, None, None)?;
    lat_var.put_values(&lat_values, None, None)?;
    time_var.put_values(&time_values, None, None)?;

    // Create gaussian blob data
    let total_size = 3 * size.1 * size.0; // 3 time steps
    let mut data_values = Vec::with_capacity(total_size);

    // Generate a gaussian blob for each time step
    for t in 0..3 {
        // For each time step, place the blob at a different location
        let center_x = size.0 as f32 * (0.3 + 0.4 * (t as f32 / 2.0));
        let center_y = size.1 as f32 * (0.3 + 0.4 * (t as f32 / 2.0));
        let sigma_x = size.0 as f32 * 0.15;
        let sigma_y = size.1 as f32 * 0.15;

        for y in 0..size.1 {
            for x in 0..size.0 {
                // Calculate gaussian function
                let dx = (x as f32 - center_x) / sigma_x;
                let dy = (y as f32 - center_y) / sigma_y;
                let exponent = -(dx * dx + dy * dy) / 2.0;
                let value = (exponent.exp()) * (1.0 + t as f32 * 0.2);

                data_values.push(value);
            }
        }
    }

    // Write the data
    data_var.put_values(&data_values, None, None)?;

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
/// * `Result<()>` - Ok if successful, or an error
pub fn create_test_weather_nc(path: &Path) -> Result<()> {
    // Define grid size
    let lon_size = 36; // 10 degree resolution
    let lat_size = 18; // 10 degree resolution
    let time_steps = 5; // 5 time steps

    // Create a new NetCDF file
    let mut file = netcdf::create(path)?;

    // Add dimensions
    let lon_dim = file.add_dimension("lon", lon_size)?;
    let lat_dim = file.add_dimension("lat", lat_size)?;
    let time_dim = file.add_unlimited_dimension("time")?;

    // Add coordinate variables
    let mut lon_var = file.add_variable::<f32>("lon", &[&lon_dim])?;
    let mut lat_var = file.add_variable::<f32>("lat", &[&lat_dim])?;
    let mut time_var = file.add_variable::<f32>("time", &[&time_dim])?;

    // Add data variables
    let mut temp_var = file.add_variable::<f32>("temperature", &[&time_dim, &lat_dim, &lon_dim])?;
    let mut u_wind_var = file.add_variable::<f32>("u_wind", &[&time_dim, &lat_dim, &lon_dim])?;
    let mut v_wind_var = file.add_variable::<f32>("v_wind", &[&time_dim, &lat_dim, &lon_dim])?;
    let mut pressure_var =
        file.add_variable::<f32>("pressure", &[&time_dim, &lat_dim, &lon_dim])?;
    let mut precip_var =
        file.add_variable::<f32>("precipitation", &[&time_dim, &lat_dim, &lon_dim])?;

    // Add file attributes
    file.add_attribute("title", "Rossby Test Weather Data")?;
    file.add_attribute("institution", "rossby test suite")?;
    file.add_attribute("source", "Synthetic weather data for testing")?;
    file.add_attribute("references", "None")?;

    // Add coordinate attributes
    lon_var.add_attribute("units", "degrees_east")?;
    lon_var.add_attribute("long_name", "Longitude")?;
    lon_var.add_attribute("standard_name", "longitude")?;

    lat_var.add_attribute("units", "degrees_north")?;
    lat_var.add_attribute("long_name", "Latitude")?;
    lat_var.add_attribute("standard_name", "latitude")?;

    time_var.add_attribute("units", "days since 2000-01-01")?;
    time_var.add_attribute("long_name", "Time")?;
    time_var.add_attribute("calendar", "standard")?;

    // Add variable attributes
    temp_var.add_attribute("units", "K")?;
    temp_var.add_attribute("long_name", "Temperature")?;
    temp_var.add_attribute("standard_name", "air_temperature")?;

    u_wind_var.add_attribute("units", "m/s")?;
    u_wind_var.add_attribute("long_name", "Eastward Wind")?;
    u_wind_var.add_attribute("standard_name", "eastward_wind")?;

    v_wind_var.add_attribute("units", "m/s")?;
    v_wind_var.add_attribute("long_name", "Northward Wind")?;
    v_wind_var.add_attribute("standard_name", "northward_wind")?;

    pressure_var.add_attribute("units", "hPa")?;
    pressure_var.add_attribute("long_name", "Sea Level Pressure")?;
    pressure_var.add_attribute("standard_name", "air_pressure_at_sea_level")?;

    precip_var.add_attribute("units", "mm/day")?;
    precip_var.add_attribute("long_name", "Precipitation Rate")?;
    precip_var.add_attribute("standard_name", "precipitation_rate")?;

    // Create coordinate values
    let lon_values: Vec<f32> = (0..lon_size).map(|i| -180.0 + (i as f32) * 10.0).collect();
    let lat_values: Vec<f32> = (0..lat_size).map(|i| -90.0 + (i as f32) * 10.0).collect();
    let time_values: Vec<f32> = (0..time_steps).map(|i| i as f32).collect();

    // Write coordinate values
    lon_var.put_values(&lon_values, None, None)?;
    lat_var.put_values(&lat_values, None, None)?;
    time_var.put_values(&time_values, None, None)?;

    // Create weather data arrays
    let total_size = time_steps * lat_size * lon_size;
    let mut temp_data = Vec::with_capacity(total_size);
    let mut u_wind_data = Vec::with_capacity(total_size);
    let mut v_wind_data = Vec::with_capacity(total_size);
    let mut pressure_data = Vec::with_capacity(total_size);
    let mut precip_data = Vec::with_capacity(total_size);

    // Generate synthetic weather data
    for t in 0..time_steps {
        for y in 0..lat_size {
            let lat = lat_values[y];

            for x in 0..lon_size {
                let lon = lon_values[x];

                // Base temperature varies with latitude (colder at poles)
                let base_temp = 273.15 + 30.0 * (1.0 - (lat / 90.0).abs());

                // Add some longitudinal variation and time evolution
                let lon_rad = lon * PI / 180.0;
                let time_factor = t as f32 * 0.1;
                let temp = base_temp + 5.0 * (lon_rad + time_factor).sin();

                // Create wind field with some rotation
                let u_wind = 5.0 * (lat * PI / 180.0).cos() + 2.0 * (lon_rad + time_factor).sin();
                let v_wind = 2.0 * (lon_rad + time_factor).cos();

                // Pressure field with high/low pressure systems
                let pressure_base = 1013.25; // Standard sea level pressure
                let pressure_var =
                    15.0 * (lon_rad * 2.0 + time_factor).sin() * (lat * PI / 180.0).cos();
                let pressure = pressure_base + pressure_var;

                // Precipitation tends to be higher in tropics and where pressure is lower
                let precip_base = 2.0 * (1.0 - 2.0 * (lat / 45.0).abs().min(1.0).powi(2));
                let precip_var = 3.0 * (pressure_var < 0.0) as i32 as f32 * (-pressure_var / 15.0);
                let precip = (precip_base + precip_var).max(0.0); // No negative precipitation

                // Add data to arrays
                temp_data.push(temp);
                u_wind_data.push(u_wind);
                v_wind_data.push(v_wind);
                pressure_data.push(pressure);
                precip_data.push(precip);
            }
        }
    }

    // Write the data variables
    temp_var.put_values(&temp_data, None, None)?;
    u_wind_var.put_values(&u_wind_data, None, None)?;
    v_wind_var.put_values(&v_wind_data, None, None)?;
    pressure_var.put_values(&pressure_data, None, None)?;
    precip_var.put_values(&precip_data, None, None)?;

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

        // Verify we can open and read the file
        let nc_file = netcdf::open(&file_path).unwrap();
        assert!(nc_file.variable("gradient").is_some());
        assert_eq!(nc_file.dimension("lon").unwrap().len(), 10);
        assert_eq!(nc_file.dimension("lat").unwrap().len(), 10);
    }

    #[test]
    fn test_create_sinusoidal_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("sinusoidal.nc");

        assert!(create_sinusoidal_nc(&file_path, (10, 10)).is_ok());
        assert!(file_path.exists());

        // Verify we can open and read the file
        let nc_file = netcdf::open(&file_path).unwrap();
        assert!(nc_file.variable("wave").is_some());
        assert_eq!(nc_file.dimension("lon").unwrap().len(), 10);
        assert_eq!(nc_file.dimension("lat").unwrap().len(), 10);
    }

    #[test]
    fn test_create_gaussian_blob_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("gaussian_blob.nc");

        assert!(create_gaussian_blob_nc(&file_path, (10, 10)).is_ok());
        assert!(file_path.exists());

        // Verify we can open and read the file
        let nc_file = netcdf::open(&file_path).unwrap();
        assert!(nc_file.variable("blob").is_some());
        assert_eq!(nc_file.dimension("lon").unwrap().len(), 10);
        assert_eq!(nc_file.dimension("lat").unwrap().len(), 10);
    }

    #[test]
    fn test_create_test_weather_nc() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("weather_test.nc");

        assert!(create_test_weather_nc(&file_path).is_ok());
        assert!(file_path.exists());

        // Verify we can open and read the file
        let nc_file = netcdf::open(&file_path).unwrap();
        assert!(nc_file.variable("temperature").is_some());
        assert!(nc_file.variable("u_wind").is_some());
        assert!(nc_file.variable("v_wind").is_some());
        assert!(nc_file.variable("pressure").is_some());
        assert!(nc_file.variable("precipitation").is_some());
    }
}
