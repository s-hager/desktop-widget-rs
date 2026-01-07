# desktop-widget-rs

### Cargo commands

#### Create a new project
`cargo init` Initialize a project in your current folder

#### Build and run
`cargo run` Compiles your code and runs the resulting program immediately.

`cargo build` Compiles your code and creates an executable in target/debug/.

`cargo check` Quickly checks if your code compiles without actually building the binary (very fast).

#### Release build
`cargo build --release` To create an optimized, standalone executable without a debug console (located in: target/release/desktop-widget-rs.exe)

#### Build release action
- Tag release: Create a tag for the new version (e.g., git tag v0.1.1).
- Push Tag: git push origin v0.1.1.
- The Action will automatically run, build the release, and upload the zip file to the GitHub Release page. The app's auto-updater will then be able to detect this new version (provided the version in Cargo.toml is higher than in the current release).
