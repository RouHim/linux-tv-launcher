# Linux TV Launcher

A modern, user-friendly game launcher for Linux designed specifically for TV interfaces. Built with Rust and Iced GUI framework, featuring gamepad navigation and automatic artwork fetching.

## Features

- **Gamepad-Friendly Navigation**: Full controller support for couch gaming
- **Automatic Artwork**: Fetches game covers from multiple sources (SteamGridDB, Heroic, SearXNG)
- **Desktop Integration**: Launches games from .desktop files and Heroic Game Launcher
- **System Monitoring**: Built-in system update checker
- **Responsive UI**: Optimized for 1080p+ TV displays
- **Fast Startup**: Minimal dependencies, quick loading times

## Installation

### From Source

1. Ensure you have Rust installed (rustup recommended)
2. Clone the repository:
   ```bash
   git clone https://github.com/RouHim/linux-tv-launcher.git
   cd linux-tv-launcher
   ```
3. Build and run:
   ```bash
   cargo build --release
   ./target/release/linux-tv-launcher
   ```

### Dependencies

- GTK libraries (for Iced)
- Game controllers (optional, for gamepad support)

## Usage

Launch the application and use your gamepad or keyboard to navigate:

- **D-pad/Left Stick**: Navigate menu
- **A Button/Enter**: Select/Launch game
- **B Button/Escape**: Go back
- **Y Button**: System menu

### Game Sources

The launcher automatically detects games from:
- Desktop entries (.desktop files)
- Heroic Game Launcher library
- Steam (via desktop shortcuts)

## Configuration

Games are automatically discovered. For custom setups:

- Place .desktop files in `~/.local/share/applications/`
- Configure Heroic Launcher with your game library
- Steam games appear automatically via desktop shortcuts

## Development

### Setup

1. Clone and build as above
2. Run in development mode:
   ```bash
   cargo run
   ```

### Project Structure

- `src/main.rs`: Application entry point
- `src/app.rs`: Main application logic
- `src/ui/`: User interface components
- `src/model.rs`: Data models
- `src/storage.rs`: Persistent storage
- `assets/`: Embedded assets (fonts, icons)

### Contributing

Contributions welcome! Please:
- Follow Rust formatting (`cargo fmt`)
- Run tests (`cargo test`)
- Ensure clippy passes (`cargo clippy`)

## Support

For issues and feature requests, please use GitHub Issues.