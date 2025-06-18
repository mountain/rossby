use std::path::Path;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Path to the NetCDF file
    let file_path = Path::new("tests/fixtures/2m_temperature_1982_5.625deg.nc");
    
    println!("Inspecting NetCDF file: {}", file_path.display());
    
    // Open the NetCDF file
    let file = netcdf::open(&file_path)?;
    
    // Print file information
    println!("\n=== FILE INFORMATION ===");
    
    // Print dimensions
    println!("\nDimensions:");
    for dim in file.dimensions() {
        println!("  {} = {} {}", 
            dim.name(), 
            dim.len(), 
            if dim.is_unlimited() { "(unlimited)" } else { "" }
        );
    }
    
    // Print variables
    println!("\nVariables:");
    for var in file.variables() {
        print!("  {} ({:?})", var.name(), var.vartype());
        
        // Print dimensions for this variable
        print!(" [");
        for (i, dim) in var.dimensions().iter().enumerate() {
            if i > 0 { print!(", "); }
            print!("{} = {}", dim.name(), dim.len());
        }
        println!("]");
        
        // Print attributes for this variable
        for attr in var.attributes() {
            print!("    {}: ", attr.name());
            match attr.value() {
                Ok(val) => println!("{:?}", val),
                Err(e) => println!("Error reading value: {}", e),
            }
        }
    }
    
    // Print global attributes
    println!("\nGlobal Attributes:");
    for attr in file.attributes() {
        print!("  {}: ", attr.name());
        match attr.value() {
            Ok(val) => println!("{:?}", val),
            Err(e) => println!("Error reading value: {}", e),
        }
    }
    
    // Print information about the first data values
    println!("\nSample Data Values:");
    for var in file.variables() {
        println!("  {} ({:?}):", var.name(), var.vartype());
        
        match var.name().as_str() {
            "longitude" | "latitude" | "time" => {
                // For coordinate variables, try to print all values
                match var.vartype() {
                    netcdf::types::VariableType::Basic(netcdf::types::BasicType::Float) => {
                        println!("    Trying to read as f32...");
                        match var.get_values::<f32, _>(&[] as &[netcdf::Extent]) {
                            Ok(vals) => println!("    Values: {:?}", vals),
                            Err(e) => println!("    Error reading values: {}", e),
                        }
                    },
                    netcdf::types::VariableType::Basic(netcdf::types::BasicType::Double) => {
                        println!("    Trying to read as f64...");
                        match var.get_values::<f64, _>(&[] as &[netcdf::Extent]) {
                            Ok(vals) => println!("    Values: {:?}", vals),
                            Err(e) => println!("    Error reading values: {}", e),
                        }
                    },
                    _ => println!("    Skipping unsupported type"),
                }
            },
            _ => {
                // For data variables, try to read just a few values
                match var.vartype() {
                    netcdf::types::VariableType::Basic(netcdf::types::BasicType::Float) => {
                        println!("    Trying to read first value as f32...");
                        // Try to read just the first value
                        let indices = vec![0; var.dimensions().len()];
                        let mut index_array = [0; 10]; // Most NetCDF files won't have more than 10 dimensions
                        for j in 0..var.dimensions().len() {
                            index_array[j] = indices[j];
                        }
                        
                        match var.get_value::<f32, _>(&index_array[..var.dimensions().len()]) {
                            Ok(val) => println!("    First value: {}", val),
                            Err(e) => println!("    Error reading first value: {}", e),
                        }
                    },
                    netcdf::types::VariableType::Basic(netcdf::types::BasicType::Double) => {
                        println!("    Trying to read first value as f64...");
                        // Try to read just the first value
                        let indices = vec![0; var.dimensions().len()];
                        let mut index_array = [0; 10]; // Most NetCDF files won't have more than 10 dimensions
                        for j in 0..var.dimensions().len() {
                            index_array[j] = indices[j];
                        }
                        
                        match var.get_value::<f64, _>(&index_array[..var.dimensions().len()]) {
                            Ok(val) => println!("    First value: {}", val),
                            Err(e) => println!("    Error reading first value: {}", e),
                        }
                    },
                    _ => println!("    Skipping unsupported type"),
                }
            }
        }
    }
    
    Ok(())
}
