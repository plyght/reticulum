# Subnet Vox

A simple UDP-based multicast chat application written in Rust. This allows multiple users on the same subnet to communicate through a terminal-based interface.

This project was originally written in C++ and has been converted to Rust for improved safety, concurrency, and maintainability.

## Features

- P2P UDP messaging compatible with both local networks and Tailscale
- Automatic peer discovery
- Terminal-based UI with message history
- Cross-platform support (Linux, macOS, Windows)
- Cyberpunk-style introduction sequence
- Exit with Ctrl+Q or Ctrl+C

## Requirements

- Rust (latest stable version)
- Cargo package manager

## Building

To build the project, run:

```
./build.sh
```

Or manually with:

```
cargo build --release
```

## Running

After building, run the application:

```
./target/release/subnet_vox
```

## Usage

1. Launch the application
2. Enter your username when prompted
3. Wait for the connection to be established
4. Start chatting!

## Project Structure

- `src/main.rs` - Main entry point
- `src/message.rs` - Message data structure and encoding/decoding
- `src/networking.rs` - UDP multicast broadcasting and receiving
- `src/console_graphics.rs` - Terminal UI rendering
- `src/user_interface.rs` - User interaction handling
- `src/constants.rs` - Shared constants and configuration

## Migration Benefits

This Rust implementation offers several advantages over the original C++ version:

- Memory safety without garbage collection
- Fearless concurrency using Tokio's async/await
- Better error handling with Result types
- Centralized constants management
- Cross-platform terminal UI using Crossterm
- More modular code structure

## Implementation Notes

- This project uses Tokio for async networking
- A special Rust flag `--cfg tokio_allow_from_blocking_fd` is required when building to allow conversion of blocking sockets to Tokio sockets
- The build script automatically configures the necessary Rust flags in `.cargo/config.toml`

## License

This project is open source and available for any use.