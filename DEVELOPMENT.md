# Development
## Build
1. **Clone and Build**:
   ```bash
   git clone https://github.com/microsoft/sysinternals-listent.git
   cd sysinternals-listent
   cargo build --release
   ```
2. **Setting version**:
Do not update the version in the TOML file. Simply set an env variable called VERSION and the build system will pick it up.
   ```bash
   export VERSION=<version>
   git clone https://github.com/microsoft/sysinternals-listent.git
   cd sysinternals-listent
   cargo build --release
   ```
3. **Build Package**:

   ```bash
   # Replace 1.0.0 with the actual version from Cargo.toml
   ./makePackages.sh . target/release listent 1.0.0 0 brew ""
   ```

## Test
The project includes a comprehensive test suite located in the `tests/` directory.

