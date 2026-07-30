# hyprbinds source tree

GTK4 Hyprland ecosystem configuration studio. Binary entry: `main.rs`.

This file documents every module under `src/`. Keep it updated when modules
are added, removed, or reorganised.

## Conventions

- A plain `xxx.rs` module holds **logic** (parsers, models, IO).
- An `xxx_ui.rs` module holds **GTK4 widgets** that consume the logic.
- Files larger than ~600 lines are candidates for splitting into submodules
  (see the *Split history* section at the bottom).

## Top-level modules

| Module | Purpose |
|---|---|
| `main.rs` | CLI entry: `--health`, `--sysinfo`, `--dump`, `--export`, `--import`, then launches the GTK app. |
| `config.rs` | Loads and finalises `BindCollection` from Hyprland config (handles `.source` rewriting, Lua helpers, source-of-truth). |
| `bind.rs` | `Keybind`, `EnvVar`, `ConfigVariable`, `StartupEntry`, `Submap` data types; conflict detection. |
| `clients.rs` | `hyprctl clients` parsing — workspace, layer rule, window rule population. |
| `conflicts.rs` | Detects overlapping keybinds (modmask, submap, mouse, leader). |
| `keys.rs` | Keycode → human label tables (QMK + Hyprland). |
| `spec.rs` | `SpecItem` definitions for the Variables / Settings / etc. spec pages. |
| `variables.rs` | `ConfigVariable` template parsing and substitution (`{{var}}`). |
| `window_rules.rs` | `WindowRule` data type + parser + writer hooks. |
| `backup.rs` | Snapshot rotation: sidecar `.bak` + timestamped history in `~/.config/hyprbinds/backups/`. |
| `writer.rs` | All disk-write logic — atomic replace, managed-section markers, every `add_*` / `save_*` / `delete_*` for binds/vars/env/startup/submaps/rules. |
| `bundle.rs` | JSON bundle import/export (with merge + conflict policy). |
| `bundle_ui.rs` | GTK import/export dialog. |
| `dialog.rs` | Reusable GTK dialog primitives (modal builders, error popup). |
| `debounce.rs` | `Debouncer` helper used to coalesce rapid edits. |
| `palette.rs` | Theme palette tokens (light/dark/accent variants). |
| `ui_prefs.rs` | Persisted UI preferences (last active tab, splitter ratios, etc.). |
| `settings_config.rs` | `~/.config/hyprbinds/settings.toml` schema and load/save. |
| `nav.rs` | Sidebar navigation graph — resolves ids to page destinations. |
| `sysinfo.rs` | Hardware/software report generator for bug reports (CLI: `--sysinfo`). |
| `health.rs` | System diagnostic probes (pipewire, portals, nvidia, etc.) — CLI: `--health`. |
| `lua/collect_binds.lua` | Lua helper executed by Hyprland to dump live bind state. |

## Studios (companion config editors)

Each studio follows the same pattern: `xxx.rs` (logic), `xxx_ui.rs` (GTK page),
and optional `xxx_*` helpers.

### Hyprland core

| Module | Purpose |
|---|---|
| `ui.rs` | Main window builder — top bar, sidebar, hub notebook, hub page routing. |
| `overview_ui.rs` | Overview dashboard (counts, recent files, quick actions). |
| `dispatchers.rs` | Dispatcher catalog (General / Window / Workspace / Group / Cursor categories + field schema). |
| `dispatcher_ui.rs` | GTK page for browsing and editing dispatchers. |
| `env.rs` / `env_ui.rs` | `env =` editor. |
| `lookfeel_ui.rs` | Animations / curves / gestures / decorations page. |
| `curve_editor.rs` | Custom GTK widget for bezier + spring animation curves. |
| `logs.rs` / `logs_ui.rs` | `journalctl --user` viewer with filter and follow. |
| `screenshare_ui.rs` | Screenshare portal status + helpers. |
| `startup.rs` / `startup_ui.rs` | `exec-once` / `exec` startup entries. |
| `health_ui.rs` | GTK page that mirrors the CLI `--health` output. |

### Waybar studio

| Module | Purpose |
|---|---|
| `waybar.rs` | Load `~/.config/waybar/config.jsonc` and `style.css`; reload via `SIGUSR2`. |
| `waybar_model.rs` | In-memory model of bar (zones, modules, positions). |
| `waybar_modules.rs` | Built-in module catalog + field schema. |
| `waybar_style.rs` | CSS token parsing + section injection. |
| `waybar_themes.rs` | Bundled theme presets (gruvbox, catppuccin, …). |
| `waybar_ui.rs` | Studio page — preview, layout tree, module form, style editor. |

### Rofi studio

| Module | Purpose |
|---|---|
| `rofi.rs` | Load `~/.config/rofi/config.rasi` and `hyprbinds-theme.rasi`. |
| `rofi_model.rs` | AST for RASI properties (color, padding, …). |
| `rofi_theme.rs` | Theme preset catalog. |
| `rofi_ui.rs` | Studio page (theme picker + live preview). |
| `rofi_apps.rs` | Managed launcher apps (clipboard, power, wifi, …). |
| `rofi_apps_ui.rs` | GTK page for enabling/disabling + configuring apps. |

### Starship studio

| Module | Purpose |
|---|---|
| `starship.rs` | Load `$STARSHIP_CONFIG` (default `~/.config/starship.toml`). |
| `starship_model.rs` | Schema-aware prompt model. |
| `starship_options.rs` | 30 curated fields (prompt, character, git, time, …). |
| `starship_ui.rs` | Studio page (Options / Config / Shells tabs). |

### Wallpaper

| Module | Purpose |
|---|---|
| `wallpaper.rs` | `awww` integration (load, list, set, transitions). |
| `wallpaper_ui.rs` | GTK page — image picker, mode (fill/fit/tile/span), per-output. |

### Audio

| Module | Purpose |
|---|---|
| `audio.rs` | `pactl` / `wpctl` / `amixer` wrappers for sinks, sources, streams. |
| `audio_ui.rs` | GTK page — volume sliders, default sink/source, per-app. |

### VIA / Vial (USB HID keymap editor)

| Module | Purpose |
|---|---|
| `via.rs` | Device discovery + protocol over USB HID; sideloaded VIA definition JSON. |
| `via_studio.rs` | High-level "Studio" wrapper (board picker, lighting, macros). |
| `via_studio_ui.rs` | GTK Studio page. |
| `via_ui.rs` | Keymap matrix UI — layer picker, layout preview, per-key remap. |
| `via_vial_ui.rs` | Vial-specific widgets — tap dance, combos, key overrides. |

### Misc

| Module | Purpose |
|---|---|
| `extra_ui.rs` | Rules + Settings group: workspace rules, layer rules, monitors, devices, animations, gestures. |

## Split history

The following files were consolidated into per-feature subdirectories during
the modularisation refactor (post initial commit):

- `ui.rs` → `src/ui/` (nav, hub, dialogs, lists, row renderers)
- `waybar_ui.rs` → `src/waybar/ui/` (preview, modules, style, themes)
- `extra_ui.rs` → `src/extra/ui/` (workspace rules, layer rules, monitors, devices, animations, gestures)
- `writer.rs` → `src/writer/` (binds, vars, env, startup, submaps, rules, spec)
- `dispatchers.rs` → `src/dispatchers/` (catalog, schema, ui)
- `rofi_ui.rs` → `src/rofi/ui/`
- `via*.rs` → `src/via/`

See `git log -- src/` for per-file split commits.
