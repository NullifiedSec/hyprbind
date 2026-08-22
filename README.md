# Hyprbinds

**Edit Hyprland binds, rules, and settings safely.**

Hyprbinds is a GTK4 configuration studio for [Hyprland](https://hyprland.org/). It loads your Lua config, lets you change keybinds, rules, look & feel, monitors, and more, then writes managed sections back with automatic backups.

Companion tools (Waybar, Wallpaper, Starship, VIA, Audio, Screenshare) live under an **Experimental** sidebar section and stay hidden until you enable **Developer mode** on the Health page.

## Features

### Core (always available)

- Keybinds, variables, environment, submaps, startup
- Window / workspace / layer rules
- Look & Feel, monitors, devices, animations, curves, gestures
- Session health checks, logs, JSON import/export
- Safe writes with rotating backups

### Experimental (Developer mode)

- Waybar layout & style
- Wallpaper (`awww`)
- Starship prompt
- VIA / Vial USB keymap
- Audio panel & screenshare helpers

## Build

See [DEPENDENCIES.md](DEPENDENCIES.md) for distro packages.

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

Optional desktop entry (from repo root):

```bash
cp packaging/dev.hyprbinds.Hyprbinds.desktop ~/.local/share/applications/
```

## CLI

| Flag | Description |
|---|---|
| *(none)* | Launch the GTK app |
| `--health` | Print session diagnostics |
| `--sysinfo` | Print hardware/software dump |
| `--dump` / `--json` | Summarize loaded Hyprland config |
| `--export <path>` | Write `hyprbinds-export.json` |
| `--import <path>` | Restore from export JSON |

## Screenshots

Add PNGs under [`assets/screenshots/`](assets/screenshots/) when ready for release marketing.

## License

MIT — see [LICENSE](LICENSE).
