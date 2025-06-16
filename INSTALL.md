# Installation Guide for rossby

## Prerequisites

### macOS

Install the required dependencies using Homebrew:

```bash
brew install hdf5 netcdf
```

### Ubuntu/Debian

Install the required dependencies using apt:

```bash
sudo apt-get update
sudo apt-get install -y libnetcdf-dev libhdf5-dev
```

### Other Linux Distributions

Install the NetCDF and HDF5 development packages using your distribution's package manager.

## Building from Source

Once you have the prerequisites installed:

```bash
# Clone the repository
git clone https://github.com/mountain/rossby.git
cd rossby

# Build the project
cargo build --release

# Run tests
cargo test

# Install the binary
cargo install --path .
```

## Verifying Installation

After installation, you can verify rossby is working:

```bash
# Check version
rossby --version

# View help
rossby --help
```

## Troubleshooting

### HDF5 Not Found

If you get an error about HDF5 not being found during compilation:

1. Make sure HDF5 is installed (`brew list hdf5` on macOS)
2. You may need to set environment variables:
   ```bash
   export HDF5_DIR=$(brew --prefix hdf5)  # macOS
   export HDF5_DIR=/usr  # Ubuntu/Debian
   ```

### NetCDF Not Found

Similar to HDF5, ensure NetCDF is properly installed and the library can be found by the linker.
