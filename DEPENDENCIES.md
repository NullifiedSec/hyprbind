# Dependencies

Runtime and build dependencies for **hyprbinds**. Distro package names below are Arch/CachyOS-oriented; equivalents exist on other distros.

## Build (Rust / GTK)

| Package | Why |
|---|---|
| `rust` / `cargo` | Compile the app |
| `gtk4` | UI toolkit |
| `pkgconf` | Find GTK via pkg-config |
| `gcc` / `base-devel` | Link C dependencies |

Cargo crates (see `Cargo.toml`): `gtk4`, `serde`, `serde_json`, `dirs`, `thiserror`, `hidapi`, `via-protocol`.

```bash
sudo pacman -S rust gtk4 pkgconf base-devel
cargo build --release
```

## VIA hardware keymap

Talks to VIA-enabled QMK keyboards over USB HID (usage page `0xFF60`). On Linux you need read/write access to the keyboard's `hidraw` node — either run as a user in the right group or install a udev rule, then re-plug:

```bash
# Example udev rule (adjust idVendor/idProduct or match usage page)
echo 'KERNEL=="hidraw*", ATTRS{idVendor}=="342d", MODE="0666"' | sudo tee /etc/udev/rules.d/99-via-hid.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Keyboard definition JSON files live in `~/.config/hyprbinds/via-definitions/` (sideload via **Load definition…**, same format as VIA's Design tab). Bundled examples ship in the repo `via-definitions/` folder.
## Core runtime (Hyprland config editor)

| Package | Why |
|---|---|
| `hyprland` | Compositor + `hyprctl` (load monitors/clients, health) |
| `lua` | Your Hyprland Lua config stack (user environment) |

Optional but useful while editing:

| Package | Why |
|---|---|
| `wl-clipboard` | System clipboard helpers (GTK clipboard still works without it) |

## System Health checks (Screenshare / Audio / Session)

These are **not linked into the binary**. Health probes them with `systemctl`, `pgrep`, `pactl`, and filesystem checks. Install what you use:

| Package | Why |
|---|---|
| `xdg-desktop-portal` | Desktop portals (file pickers, screenshare) |
| `xdg-desktop-portal-hyprland` | Hyprland screenshare / portal backend |
| `xdg-desktop-portal-gtk` | Fallback portal (file dialogs) — recommended |
| `pipewire` | Audio + screenshare media |
| `wireplumber` | PipeWire session manager |
| `pipewire-pulse` | PulseAudio compatibility (`pactl`) |
| `pipewire-alsa` | ALSA compatibility (optional) |
| `libpulse` | Provides `pactl` on many setups |
| `systemd` | User units (`systemctl --user`) for service status |

Recommended one-liner (Arch):

```bash
sudo pacman -S xdg-desktop-portal xdg-desktop-portal-hyprland xdg-desktop-portal-gtk \
  pipewire wireplumber pipewire-pulse pipewire-alsa libpulse
```

Enable user audio services if needed:

```bash
systemctl --user enable --now pipewire.service pipewire-pulse.service wireplumber.service
```

Optional portal preference (Health can suggest this):

```bash
mkdir -p ~/.config/xdg-desktop-portal
printf '%s\n' '[preferred]' 'default=hyprland;gtk' \
  > ~/.config/xdg-desktop-portal/hyprland-portals.conf
systemctl --user restart xdg-desktop-portal xdg-desktop-portal-hyprland
```

## NVIDIA (optional)

| Package / note | Why |
|---|---|
| Proprietary NVIDIA drivers (up to date) | Hyprland + screenshare on NVIDIA |
| Hyprland NVIDIA wiki env hints | Only if you hit black screens / share failures |

Health reports NVIDIA presence as an informational hint; it does not install drivers.

## Audio panel

| Package | Why |
|---|---|
| `pipewire` / `pipewire-pulse` / `wireplumber` | Volume routing |
| `libpulse` (`pactl`) | Sink/source/stream control |
| `wireplumber` (`wpctl`) | Preferred volume get/set |
| `alsa-utils` (`amixer`, `alsamixer`) | ALSA mixer + terminal TUI |
| A terminal (`kitty`, `foot`, `alacritty`, …) or `$TERMINAL` | Launch alsamixer |

## Wallpaper (awww)

| Package | Why |
|---|---|
| `awww` | Wallpaper daemon + CLI (`awww-daemon`, `awww img`) |

Start daemon once (or add to Startup):

```bash
awww-daemon
# or in Hyprbinds Startup: awww-daemon
```

## Waybar Studio

| Package | Why |
|---|---|
| `waybar` | Status bar process (reload via `SIGUSR2`) |

Config files edited by the Waybar Studio page:

- `~/.config/waybar/config.jsonc` (or `config` / `config.json`)
- `~/.config/waybar/style.css`

Apply snapshots both files before writing (same backup system as Hyprland config). Included module files are discovered for the UI but not rewritten.

```bash
sudo pacman -S waybar
```

## Rofi Studio

| Package | Why |
|---|---|
| `rofi` | Application launcher / dmenu replacement |

Config files edited by the Rofi Studio page:

- `~/.config/rofi/config.rasi`
- `~/.config/rofi/hyprbinds-theme.rasi` (managed theme referenced via `@theme`)

Apply snapshots both files before writing (same backup system as Hyprland config). Use **Test Rofi** to launch a live demo with the current theme.

```bash
sudo pacman -S rofi
```

## Rofi Apps

Managed launcher utilities (clipboard, power menu, Wi-Fi, custom scripts) live under **Desktop → Rofi Apps**.

| Package | Why |
|---|---|
| `rofi` | Menu front-end for every app |
| `cliphist` | Clipboard history store (clipboard app) |
| `wl-clipboard` | `wl-copy` / `wl-paste` (clipboard app + watcher) |
| `networkmanager` | `nmcli` for the Wi-Fi picker (optional) |

Files written by **Apply**:

- `~/.config/hyprbinds/rofi-apps.json` — enabled apps and overrides
- `~/.config/rofi/scripts/*.sh` — executable scripts

Clipboard also needs a Startup daemon that watches **text and images**:

```bash
sh -c 'wl-paste --type text --watch cliphist store & exec wl-paste --type image --watch cliphist store'
```

Use **Add daemon…** on the clipboard app (or add that under Startup). The clipboard script opens Rofi in icon mode so PNG/JPEG/etc. entries show thumbnails.

```bash
sudo pacman -S cliphist wl-clipboard
```

## Starship Studio

| Package | Why |
|---|---|
| `starship` | Cross-shell prompt (`starship init`, presets, preview) |

Config edited by the Starship Studio page:

- `~/.config/starship.toml` (or `$STARSHIP_CONFIG`)

**Options** tab exposes 30 curated schema fields (prompt globals, character, directory, git, duration, identity, time). **Config** remains the full raw TOML editor. **Shells** detects installed shells and can Enable/Disable a Hyprbinds-managed init block. Apply snapshots before writing.

```bash
sudo pacman -S starship
```


Every write snapshots the previous file to:

- sidecar: `hyprland.lua.hyprbinds.bak`
- history: `~/.config/hyprbinds/backups/<file>.<timestamp>.lua` (keeps 12)

**Restore** (top bar or Ctrl+P → “Restore last good config”) copies the newest snapshot back.


| Package | Why |
|---|---|
| `systemd` | `journalctl --user` for the Logs viewer |

No extra packages beyond a normal systemd user session.


Optional helpers used when collecting a pasteable hardware/software dump:

| Package | Why |
|---|---|
| `pciutils` (`lspci`) | GPU + PCI device IDs |
| `usbutils` (`lsusb`) | USB peripheral IDs |
| `util-linux` (`lscpu`, `lsblk`, `uname`) | CPU / disks / kernel |
| `procps-ng` / `procps` (`free`) | Memory summary |
| `systemd` (`hostnamectl`) | Distro / firmware summary |
| `libpulse` (`pactl`) | Audio server + sinks |
| `pacman` (Arch) / `dpkg` / `rpm` | Package version queries |

DMI board/BIOS fields are read from `/sys/class/dmi/id` (no `dmidecode` required). Serial numbers and product UUIDs are omitted from the report.

```bash
sudo pacman -S pciutils usbutils util-linux procps-ng libpulse
```
