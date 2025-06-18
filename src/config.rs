//! Configuration management for rossby.
//!
//! This module handles the layered configuration system with the following precedence:
//! 1. Command-line arguments (highest priority)
//! 2. Environment variables
//! 3. JSON config file
//! 4. Default values (lowest priority)

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{Result, RossbyError};

/// Command-line arguments for rossby
#[derive(Parser, Debug)]
#[command(name = "rossby")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the NetCDF file to serve
    pub netcdf_file: PathBuf,

    /// Host address to bind to
    #[arg(short = 'H', long, env = "ROSSBY_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, env = "ROSSBY_PORT", default_value = "8000")]
    pub port: u16,

    /// Number of worker threads
    #[arg(short, long, env = "ROSSBY_WORKERS")]
    pub workers: Option<usize>,

    /// Path to JSON configuration file
    #[arg(short, long, env = "ROSSBY_CONFIG")]
    pub config: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "ROSSBY_LOG_LEVEL", default_value = "info")]
    pub log_level: String,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host address to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Number of worker threads (None = number of CPU cores)
    #[serde(default)]
    pub workers: Option<usize>,
}

/// Data processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    /// Default interpolation method
    #[serde(default = "default_interpolation")]
    pub interpolation_method: String,

    /// Path to the NetCDF file
    #[serde(default)]
    pub file_path: Option<PathBuf>,
}

/// Complete configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Data configuration
    #[serde(default)]
    pub data: DataConfig,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Config {
    /// Load configuration from all sources with proper precedence
    pub fn load() -> Result<(Self, PathBuf)> {
        let args = Args::parse();

        // Start with defaults
        let mut config = Config::default();

        // Load from JSON file if provided
        if let Some(config_path) = &args.config {
            let json_config = Self::load_from_file(config_path)?;
            config.merge(json_config);
        }

        // Override with command-line arguments
        config.server.host = args.host;
        config.server.port = args.port;
        if args.workers.is_some() {
            config.server.workers = args.workers;
        }
        config.log_level = args.log_level;

        // NetCDF file path from command line takes precedence
        let netcdf_path = args.netcdf_file;

        Ok((config, netcdf_path))
    }

    /// Load configuration from a JSON file
    fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Merge another config into this one (other takes precedence)
    fn merge(&mut self, other: Config) {
        self.server.host = other.server.host;
        self.server.port = other.server.port;
        if other.server.workers.is_some() {
            self.server.workers = other.server.workers;
        }
        self.data = other.data;
        self.log_level = other.log_level;
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server host (must be a valid IP or hostname)
        if self.server.host.is_empty() {
            return Err(RossbyError::Config {
                message: "Server host cannot be empty".to_string(),
            });
        }

        // Validate port (0 is not a valid port for users)
        if self.server.port == 0 {
            return Err(RossbyError::Config {
                message: "Server port cannot be 0".to_string(),
            });
        }

        // Validate log level
        match self.log_level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                return Err(RossbyError::Config {
                    message: format!(
                        "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
                        self.log_level
                    ),
                });
            }
        }

        // Validate interpolation method
        match self.data.interpolation_method.as_str() {
            "nearest" | "bilinear" | "bicubic" => {}
            _ => {
                return Err(RossbyError::Config {
                    message: format!(
                        "Invalid interpolation method: {}. Must be one of: nearest, bilinear, bicubic",
                        self.data.interpolation_method
                    )
                });
            }
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            data: DataConfig::default(),
            log_level: default_log_level(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: None,
        }
    }
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            interpolation_method: default_interpolation(),
            file_path: None,
        }
    }
}

// Default value functions for serde
fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_interpolation() -> String {
    "bilinear".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.data.interpolation_method, "bilinear");
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config::default();
        let mut config2 = Config::default();

        config2.server.port = 9000;
        config2.server.workers = Some(4);

        config1.merge(config2);

        assert_eq!(config1.server.port, 9000);
        assert_eq!(config1.server.workers, Some(4));
    }

    #[test]
    fn test_config_validation() {
        // Valid config should pass
        let config = Config::default();
        assert!(config.validate().is_ok());

        // Test invalid host
        let mut config = Config::default();
        config.server.host = "".to_string();
        assert!(config.validate().is_err());

        // Test invalid port
        let mut config = Config::default();
        config.server.port = 0;
        assert!(config.validate().is_err());

        // Test invalid log level
        let mut config = Config::default();
        config.log_level = "invalid".to_string();
        assert!(config.validate().is_err());

        // Test invalid interpolation method
        let mut config = Config::default();
        config.data.interpolation_method = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}
