# Hyprbind

A native GTK4 configuration studio for [Hyprland](https://hypr.land/) — built to make editing keybinds, rules, monitors, appearance, and the rest of your Hyprland setup easier without giving up control of the underlying configuration.

> **Early development:** Hyprbind is usable, but it is still evolving. Keep your configuration under version control or maintain your own backups when testing development builds.

## What is Hyprbind?

Hyprland configuration is powerful, but a sufficiently large setup eventually becomes a collection of binds, rules, variables, monitor declarations, animations, and supporting tools spread across configuration files.

Hyprbind puts a graphical workspace in front of that configuration while keeping the config itself as the source of truth. It parses your existing setup, exposes supported settings through native GTK4 interfaces, and writes managed changes back with automatic backups.

It is not intended to replace Hyprland's configuration language. The goal is to make the parts you change often easier to inspect and edit while leaving the underlying configuration accessible.

## Features

### Hyprland configuration

- Keybinds and submaps
- Window, workspace, and layer rules
- Variables and environment configuration
- Startup entries
- Monitors and input devices
- Look & Feel settings
- Animations and Bezier curves
- Gestures
- Configuration health checks and logs
- JSON import/export
- Rotating backups before managed writes

### Experimental tools

Hyprbind also contains integrations that go beyond Hyprland itself. They currently live behind **Developer mode** and should be treated as experimental:

- Waybar layout and styling
- Wallpaper management with `awww`
- Starship prompt configuration
- VIA / Vial keyboard configuration over USB
- Audio controls
- Screenshare helpers

Developer mode can be enabled from the **Health** page.

## Building from source

Hyprbind is currently built and tested as a source project rather than distributed as a stable packaged release.

See [`DEPENDENCIES.md`](DEPENDENCIES.md) for the full dependency notes. On Arch-based systems, the basic development dependencies can be installed with:

```bash
sudo pacman -S rust gtk4 pkgconf base-devel
```

Then build Hyprbind:

```bash
git clone https://github.com/NullifiedSec/hyprbind.git
cd hyprbind
cargo build --release
```

Run it with:

```bash
cargo run --release
```

or run the built binary directly:

```bash
./target/release/hyprbinds
```

An optional desktop entry is included:

```bash
cp packaging/dev.hyprbinds.Hyprbinds.desktop ~/.local/share/applications/
```

## CLI

Hyprbind also exposes a small command-line interface for diagnostics and configuration transfer.

| Flag | Description |
| --- | --- |
| *(none)* | Launch the GTK application |
| `--health` | Print session diagnostics |
| `--sysinfo` | Print a hardware/software information dump |
| `--dump` / `--json` | Summarize the loaded Hyprland configuration |
| `--export <path>` | Export configuration data as JSON |
| `--import <path>` | Restore configuration data from an export |

## Safety and backups

Configuration editors should not turn one bad click into a ruined desktop session. Hyprbind performs managed writes with rotating backups, but development software can still contain bugs.

For important setups, keep your Hyprland configuration in Git or another independent backup system as well.

## Screenshots

Screenshots are coming as the public-facing UI settles down.

## Contributing

Issues, bug reports, testing, patches, and focused improvements are welcome.

Before making a substantial change, please open an issue or discussion first so work does not accidentally head in incompatible directions. Experimental integrations in particular may change significantly while their interfaces settle.

## Project status

Hyprbind is under active development. Interfaces, supported configuration options, and experimental integrations may change between revisions. There is not yet a stable release compatibility guarantee.

## License

Hyprbind is **source-available**, not MIT-licensed and not conventional open-source software under an OSI-approved license.

The [Hyprbind Source License](LICENSE) permits use, inspection, modification, contribution, and redistribution subject to its terms, including preservation of attribution and restrictions on rebranding the project as independently originated software.

Read [`LICENSE`](LICENSE) before redistributing Hyprbind or a modified version.
