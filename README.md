<p align="center">
  <img src="assets/banner.png" alt="RhincoTV" width="100%">
</p>

<p align="center">
  <a href="https://github.com/RouHim/rhinco-tv/actions/workflows/ci.yml"><img src="https://github.com/RouHim/rhinco-tv/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/RouHim/rhinco-tv/releases/latest"><img src="https://img.shields.io/github/v/release/RouHim/rhinco-tv" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License"></a>
  <img src="https://img.shields.io/badge/platform-linux-lightgrey" alt="Platform">
  <img src="https://img.shields.io/badge/rust-2021-orange" alt="Rust">
</p>

<p align="center">
  <strong>A fullscreen, couch-friendly launcher for Linux built with Rust and Iced.</strong><br>
  Prioritizes gamepad navigation, scans popular game libraries, and provides system tools tailored for living-room setups.
</p>

---

## Features

- **Game discovery** from Steam libraries and Heroic (Epic, GOG, Amazon, sideloaded).
- **App picker** for XDG `.desktop` apps (including Flatpak and Snap exports).
- **N64 support** via mupen64plus: install `mupen64plus-qt`, then set your ROM directory in **Settings → Paths** so RhincoTV can scan it.
- **SNES support** via snes9x: install `snes9x`, configure your ROM directory within snes9x, then RhincoTV will read it from `~/.config/snes9x/snes9x.conf` or `~/.snes9x/snes9x.conf` (looks for `LastDirectory` in `[Files]` section)
- **Cover art pipeline** with Heroic art, SteamGridDB (optional API key), and SearXNG fallback.
- **Gamepad-first navigation** with keyboard shortcuts, haptics, and battery indicators.
- **System category** for updates, system info, suspend/shutdown, and exiting the launcher.
- **On-screen keyboard integration** for GNOME, KDE, wvkbd, and Squeekboard.
- **Self-updater** that checks GitHub releases on startup.

## Installation

### Download Pre-built Binaries

Download the latest release for your architecture from [GitHub Releases](https://github.com/RouHim/rhinco-tv/releases/latest):

| Architecture | Binary |
|--------------|--------|
| x86_64 | `rhinco-tv-x86_64-unknown-linux-gnu` |
| ARM64 | `rhinco-tv-aarch64-unknown-linux-gnu` |

```bash
# Example: Download and run (x86_64)
chmod +x rhinco-tv-x86_64-unknown-linux-gnu
./rhinco-tv-x86_64-unknown-linux-gnu
```

### From Source

1. Install the Rust toolchain.
2. Clone the repository:
   ```bash
   git clone https://github.com/RouHim/rhinco-tv.git
   cd rhinco-tv
   ```
3. Build and run:
   ```bash
   cargo build --release
   ./target/release/rhinco-tv
   ```

## Runtime Notes

- **Steam games** require the `steam` client in your `PATH`.
- **Heroic games** launch via the `heroic://` protocol.
- **N64 games** require `mupen64plus` and `mupen64plus-qt`; configure your ROM directory in **Settings → Paths** inside mupen64plus-qt.
- **SNES games** require `snes9x`; RhincoTV automatically reads your ROM directory from snes9x's config file (`~/.config/snes9x/snes9x.conf` or `~/.snes9x/snes9x.conf`). Configure your ROM directory in snes9x preferences - the `LastDirectory` value in the `[Files]` section will be used. Supported ROM formats: `.sfc`, `.smc`, `.fig`, `.swc`, `.bs`, `.st`
- **System updates** currently support Arch-based tools: `pacman`, `yay`, or `paru` (with `pkexec`).
- **System info** uses common utilities such as `lspci`, `glxinfo`, `vulkaninfo`, and `gamemoded` when available.
- **On-screen keyboard** support is detected automatically (GNOME, KDE, wvkbd, Squeekboard).

## Usage

### Categories

- **Games**: automatically scanned from Steam, Heroic, N64 (mupen64plus), and SNES (snes9x).
- **Apps**: curated list of desktop apps you add via the picker.
- **System**: update, system info, suspend, shutdown, exit.

### Controls

**Gamepad**
- **A / South**: Select
- **B / East**: Back
- **X / West**: Context menu
- **Y / North**: Add app (Apps category)
- **D-pad / Left Stick**: Navigate
- **LB / LT**: Previous category
- **RB / RT**: Next category
- **Select / -**: Show controls

**Keyboard**
- **Arrow Keys**: Navigate
- **Enter**: Select
- **Escape**: Back
- **Tab**: Next category
- **C**: Context menu
- **+ / A**: Add app (Apps category)
- **-**: Show controls
- **F4**: Quit launcher

## Configuration

Configuration is stored at:

- `~/.config/com/rhinco-tv/rhinco-tv/config.json` (respects `XDG_CONFIG_HOME`)

Supported settings:

- `steamgriddb_api_key`: API key for SteamGridDB. You can also set `STEAMGRIDDB_API_KEY` as an environment variable.
- `apps`: saved app entries from the picker.
- `game_launch_history`: launch timestamps used for sorting.
