# Hyprbinds

**Edit Hyprland binds, rules, settings, and programmable-keyboard firmware from one GTK4 control center.**

Hyprbinds is a GTK4 configuration studio for Hyprland. It loads your Lua config, lets you change keybinds, rules, look & feel, monitors, and more, then writes managed sections back with automatic backups.

The Vial integration is being promoted from a disconnected experimental companion into a first-class keyboard subsystem: Hyprbinds can read a Vial keyboard's onboard definition, render its exact physical layout, read its live hardware layers, and correlate physical firmware mappings with the Hyprland binds they ultimately trigger.

## Features

### Core

- Keybinds, variables, environment, submaps, startup
- Window / workspace / layer rules
- Look & Feel, monitors, devices, animations, curves, gestures
- Session health checks, logs, JSON import/export
- Safe writes with rotating backups

### Vial / VIA keyboard control

Native Rust/GTK surfaces currently include:

- Vial USB discovery and onboard-definition fetch
- Exact physical keyboard geometry
- Dynamic keymap layer reads and live remapping
- Hyprland-aware physical-key visualizer
- SUPER / CTRL / ALT / SHIFT chord correlation
- Lighting controls
- Per-key RGB / animation Studio
- Tap Dance editor
- Combo editor
- Key Override editor
- Stock VIA definition JSON fallback
- Vial diagnostics via `--vial-status`

The unified development entry point is:

```bash
cargo run -- --vial-control-center
```

The original focused visualizer remains available while integration work continues:

```bash
cargo run -- --vial-visualizer
```

### Upstream Vial compatibility bridge

Hyprbinds targets feature parity with the official Vial GUI while adding Hyprland-aware behavior upstream Vial does not provide. During the native Rust port, a pinned official Vial checkout can be bootstrapped as a compatibility escape hatch for protocol/UI features not yet ported natively, such as macros and newer Vial additions.

```bash
bash scripts/sync-vial-upstream.sh
cargo run -- --vial-control-center
```

The compatibility checkout is pinned to official `vial-kb/vial-gui` commit:

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

Detailed attribution and upstream links live in `THIRD_PARTY_NOTICES.md`.

## License

GNU General Public License v2.0 — see `LICENSE`.
