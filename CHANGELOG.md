# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.2] - 2025-06-20

### Added
- `/data` endpoint with Apache Arrow integration for efficient data extraction
- Support for requesting specific data hyperslabs with custom dimension layouts
- Maximum data points limit for preventing excessive memory usage
- Enhanced `/metadata` endpoint to include coordinate values for each dimension

### Changed
- `/metadata` endpoint now returns coordinate values in addition to dimension names

## [0.0.1] - 2025-06-19

### Added
- Initial release of Rossby
- Core functionality for loading NetCDF files into memory
- HTTP API with support for metadata, point queries, and image generation
- Multiple interpolation methods (nearest, bilinear, bicubic)
- Colormap rendering with various built-in colormaps
- Server monitoring via heartbeat endpoint
- Flexible configuration system (CLI, environment variables, JSON)
- Support for querying by physical value and raw index

### Known Issues
- "Dimension aliases" feature is experimental and not fully tested
