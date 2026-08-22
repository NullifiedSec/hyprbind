# hyprbinds source tree

GTK4 Hyprland configuration studio. Binary entry: [`main.rs`](main.rs).

This file documents every module under `src/`. Keep it updated when modules
are added, removed, or reorganised.

## Conventions

- A plain `xxx.rs` module holds **logic** (parsers, models, IO).
- An `xxx_ui.rs` (or `ui.rs` inside a package) holds **GTK4 widgets**.
- Experimental companions live under [`experimental/`](experimental/) and are
  gated behind Developer mode in the sidebar.

## Top-level modules

| Module | Purpose |
|---|---|
| `main.rs` | CLI entry: `--health`, `--sysinfo`, `--dump`, `--export`, `--import`, then launches the GTK app. |
| `config.rs` | Loads and finalises `BindCollection` from Hyprland config. |
| `bind.rs` | Bind / env / variable / startup / submap data types. |
| `clients.rs` | `hyprctl clients` parsing. |
| `conflicts.rs` | Overlapping keybind detection. |
| `keys.rs` | Keycode → human label tables. |
| `spec.rs` | Spec items for settings fields. |
| `variables.rs` | `{{var}}` template parsing. |
| `window_rules.rs` | Window rule types. |
| `backup.rs` | Snapshot rotation. |
| `writer/` | Disk writes for managed Hyprland sections. |
| `bundle.rs` / `bundle_ui.rs` | JSON import/export. |
| `dialog.rs` | Reusable GTK dialogs. |
| `debounce.rs` | Edit coalescing. |
| `palette.rs` | Command palette. |
| `ui_prefs.rs` | Dark mode + developer mode prefs. |
| `settings_config.rs` | Hyprland settings helpers. |
| `nav.rs` | Sidebar taxonomy (core + Experimental). |
| `sysinfo.rs` | `--sysinfo` dump. |
| `health.rs` / `health_ui.rs` | Diagnostics + Developer mode toggle. |
| `lua/collect_binds.lua` | Lua helper for live bind dump. |

## Hyprland UI (always visible)

| Module | Purpose |
|---|---|
| `ui.rs` | Main window, hubs, routing (large; split deferred). |
| `overview_ui.rs` | Overview + global animation speed. |
| `dispatchers.rs` / `dispatcher_ui.rs` | Dispatcher catalog + widgets. |
| `env.rs` / `env_ui.rs` | Env + submaps. |
| `lookfeel_ui.rs` | Look & Feel. |
| `curve_editor.rs` | Bézier / spring editor. |
| `logs.rs` / `logs_ui.rs` | journalctl viewer. |
| `startup.rs` / `startup_ui.rs` | Autostart entries. |
| `extra_ui.rs` | Rules + settings notebooks (large; split deferred). |

## Experimental (`src/experimental/`)

Gated by Health → **Developer mode**.

| Package | Purpose |
|---|---|
| `experimental/waybar/` | Waybar layout / style studio |
| `experimental/wallpaper/` | `awww` wallpaper |
| `experimental/starship/` | Starship prompt |
| `experimental/via/` | VIA / Vial USB keymap |
| `experimental/audio/` | Volume / sinks |
| `experimental/screenshare/` | Portal / PipeWire helpers |

## Split history

- `writer.rs` → `src/writer/`
- Companion studios → `src/experimental/{waybar,via,wallpaper,starship,audio,screenshare}/`

**Deferred:** splitting `ui.rs` / `extra_ui.rs` into `src/ui/`.
