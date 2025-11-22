# Log Viewer 2

A fast, efficient log viewer application written in Rust using egui.

## Features

- **Fast file loading**: Efficiently handles large log files (up to 20MB+) with minimal load time
- **Real-time tail**: Watch log files update in real-time with the "Tail Log" feature (enabled by default)
- **Auto-scroll**: Automatically scrolls to the end of the file to show the latest entries (enabled by default)
- **Dual log format support**:
  - Error logs: `DD.MM.YYYY HH:MM:SS.mmm *LEVEL* [thread] class message`
  - Access logs: `IP - user DD/MMM/YYYY:HH:MM:SS +TZ "METHOD PATH HTTP/VERSION" STATUS SIZE "referer" "user-agent"`
- **Search functionality**: 
  - Case-sensitive/insensitive search
  - Regex support
  - Next/Previous navigation
  - Highlighting of matches
- **Level filtering**: Filter logs by level (Info, Warn, Error, Debug, Trace)
- **Color customization**: Configurable color palette for different log levels
- **Virtual scrolling**: Only renders visible lines for optimal performance with large files
- **Export**: Export filtered log entries to a file

## Building

### Standard Build

```bash
cargo build --release
```

### If you encounter dependency issues

If you see errors related to `mime_guess2` or Rust edition 2024, try:

1. Use the build script:
```bash
./build.sh
```

2. Or manually clean the cache:
```bash
rm -rf ~/.cargo/registry/src/index.crates.io-*/mime_guess2-*
cargo build --release
```

3. Or update Rust (recommended):
```bash
# If using Homebrew
brew upgrade rust

# Or install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

## Running

```bash
cargo run --release
```

## Usage

1. **Open a log file**: Use `File > Open File...` to select a log file
2. **Tail mode**: Toggle "Tail Log" to watch for real-time updates
3. **Auto-scroll**: Toggle "Scroll to End" to automatically scroll to new entries
4. **Search**: Click the search icon (🔍) or use `View > Show Search` to open the search panel
5. **Filter by level**: Use the level filter dropdown to show only specific log levels
6. **Configure colors**: Click the settings icon (⚙️) or use `View > Show Configuration` to customize colors
7. **Export**: Use `File > Export Filtered...` to save filtered results

## Requirements

- Rust 1.83.0 or later
- Cargo 1.83.0 or later

## Troubleshooting

If you encounter dependency issues related to `mime_guess2` or Rust edition 2024, try:

1. Update Rust: `brew upgrade rust` (if using Homebrew)
2. Or install rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Clean cargo cache: `rm -rf ~/.cargo/registry/src/index.crates.io-*/mime_guess2-*`
4. Update dependencies: `cargo update`

## Performance

The application is optimized for large files:
- For files > 10MB, only the last 2MB are loaded initially
- Virtual scrolling ensures only visible lines are rendered
- Efficient file watching for real-time updates
- Memory-mapped file reading for optimal performance

