# Hyprbinds

**Edit Hyprland binds, rules, settings, and programmable-keyboard firmware from one GTK4 control center.**

Hyprbinds is a GTK4 configuration studio for Hyprland. It loads your Lua config, lets you change keybinds, rules, look & feel, monitors, and more, then writes managed sections back with automatic backups.

The Vial integration is a first-class keyboard subsystem: Hyprbinds can ask a Vial keyboard to describe itself, render its exact physical layout, read live firmware layers, distinguish firmware-level behavior from host-visible keys, and correlate those physical mappings with the Hyprland binds they ultimately trigger.

## Features

### Core

- Keybinds, variables, environment, submaps, startup
- Window / workspace / layer rules
- Look & Feel, monitors, devices, animations, curves, gestures
- Session health checks, logs, JSON import/export
- Safe writes with rotating backups

### Vial / VIA keyboard control

Native Rust/GTK surfaces include:

- Vial USB discovery, protocol/UID detection, and onboard-definition fetch
- Exact physical keyboard geometry from the board's own Vial definition
- Dynamic keymap layer reads and live remapping
- Transparent-key resolution through lower hardware layers
- Hardware-action classification: host keys/modifiers are kept distinct from `MO`, `LT`, `MT`, Tap Dance, macros, custom/opaque firmware actions, and disabled keys
- Hyprland-aware physical-key visualizer with SUPER / CTRL / ALT / SHIFT chord construction
- Hyprland variable expansion and submap-aware bind matching
- Bound-key highlighting and duplicate-chord/conflict highlighting
- Add/edit Hyprland binds directly from a physical key through the normal atomic backup writer
- Lighting controls
- Per-key RGB / animation Studio
- Tap Dance editor
- Combo editor
- Key Override editor
- Advanced macro read/edit/write/reset, including Vial v2+ Tap/Down/Up/Delay bytecode and extended 16-bit keycodes
- Interruption-safe macro writes using QMK's invalid-buffer sentinel protocol
- Vial-native encoder read/write across layers
- Alt Repeat read/write with all Vial option flags
- Vial QMK Settings discovery, read/write for known wire widths, and reset
- Vial security status, required-unlock-key display, unlock progress, and explicit lock
- Safe live matrix tester gated on Vial protocol support and unlocked state
- Stock VIA definition JSON fallback for non-self-describing VIA boards
- Vial diagnostics via `--vial-status`

The unified keyboard entry point is:

```bash
cargo run -- --vial-control-center
```

The focused Hyprland/keyboard visualizer is also available:

```bash
cargo run -- --vial-visualizer
```

### Pinned upstream Vial reference

The planned native Hyprbinds Vial feature surface no longer relies on the official GUI as a parity fallback. A pinned official Vial checkout is still supported as a reference/compatibility companion for firmware-specific behavior outside Hyprbinds' declared scope and for comparing protocol behavior during development.

```bash
bash scripts/sync-vial-upstream.sh
cargo run -- --vial-control-center
```

The checkout is pinned to official `vial-kb/vial-gui` commit:

```text
aef8222a2d0429a183b2ed692d5f9efcfd383f08
```

It is intentionally ignored by the Hyprbinds repository so upstream history and licensing remain intact. See `THIRD_PARTY_NOTICES.md`.

### Experimental desktop companions

- Waybar layout & style
- Wallpaper (`awww`)
- Starship prompt
- Audio panel & screenshare helpers

## Build

See `DEPENDENCIES.md` for distro packages.

```bash
sudo pacman -S rust gtk4 pkgconf base-devel
cargo build --release
```

Run:

```bash
cargo run --release
# or
./target/release/hyprbinds
```

Optional desktop entry:

```bash
cp packaging/dev.hyprbinds.Hyprbinds.desktop ~/.local/share/applications/
```

## CLI

| Flag | Description |
|---|---|
| *(none)* | Launch the GTK app |
| `--health` | Print session diagnostics |
| `--sysinfo` | Print hardware/software dump |
| `--vial-status` | Probe Vial hardware and print live board metadata |
| `--vial-visualizer` | Launch focused Vial/Hyprland keybind visualizer |
| `--vial-control-center` | Launch unified Vial firmware + Hyprland control center |
| `--dump` / `--json` | Summarize loaded Hyprland config |
| `--export <path>` | Write `hyprbinds-export.json` |
| `--import <path>` | Restore from export JSON |

## Upstream credits

Hyprbinds stands on a very good pile of open-source giants. Vial is the canonical Vial-firmware compatibility target; Vial itself is built around the QMK ecosystem, and Hyprbinds also supports VIA dynamic-keymap compatibility. Hyprbinds is not an official Vial, VIA, QMK, or Hyprland project.

Detailed attribution, pinned references, and upstream links live in `THIRD_PARTY_NOTICES.md`.

## License

GNU General Public License v2.0 — see `LICENSE`.
