# Publishing Checklist for Rossby v0.0.1

This document outlines the steps that have been completed to prepare the Rossby crate for publication, as well as the final steps needed to publish to crates.io.

## Completed Preparations

1. **Cargo.toml Updates**
   - [x] Set version to 0.0.1
   - [x] Added documentation field pointing to docs.rs
   - [x] Added readme field pointing to README.md
   - [x] Added exclude patterns to reduce package size
   - [x] Verified all dependencies have appropriate version constraints

2. **License Files**
   - [x] Created LICENSE-MIT (renamed from LICENSE)
   - [x] Created LICENSE-APACHE
   - [x] Created LICENSE file explaining dual licensing

3. **Documentation**
   - [x] Added CHANGELOG.md for tracking version changes
   - [x] Added CONTRIBUTING.md with development guidelines
   - [x] Updated README.md with:
     - [x] Early development status notice
     - [x] Version consistency (0.0.1)
     - [x] Fixed formatting issues
     - [x] Added documentation for "querying by physical value" feature
     - [x] Removed references to untested features
     - [x] Improved installation and usage examples

4. **Test Data Corrections**
   - [x] Fixed the test fixture (2m_temperature_1982_5.625deg.nc):
     - [x] Corrected time definition to start at 1982-01-01 00:00:00
     - [x] Set time to be days since the start date
     - [x] Flipped data both vertically and horizontally for correct geospatial representation
     - [x] Created scripts in scripts/ directory for data inspection and correction

5. **Verification**
   - [x] Ran `cargo package --list --allow-dirty` to verify included files
   - [x] Ran `cargo publish --dry-run --allow-dirty` to check for publishing issues

## Final Publishing Steps

1. **Commit Changes**
   ```bash
   git add .
   git commit -m "chore: prepare v0.0.1 release"
   ```

2. **Create Git Tag**
   ```bash
   git tag -a v0.0.1 -m "v0.0.1"
   git push origin v0.0.1
   ```

3. **Publish to crates.io**
   ```bash
   # Login to crates.io (only needed once)
   cargo login

   # Publish
   cargo publish
   ```

4. **Post-Publishing**
   - Verify the package appears on crates.io
   - Check that docs.rs successfully generated documentation
   - Announce the release in relevant channels if appropriate

## Future Release Recommendations

1. **Before v0.1.0**
   - Complete and thoroughly test the "dimension aliases" feature
   - Gather user feedback on API design
   - Improve test coverage

2. **For v0.1.0**
   - Consider it the first "stable" pre-release
   - Ensure all planned features for the initial release are complete
   - Freeze the core API design (minimize breaking changes after this)

3. **Towards v1.0.0**
   - Only release v1.0.0 when the API is considered stable
   - Ensure comprehensive documentation
   - Provide migration guides if any breaking changes from v0.x
