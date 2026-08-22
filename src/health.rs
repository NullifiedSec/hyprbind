//! Desktop health checks — Hyprland session, portals, PipeWire, and related tools.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Fail,
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
            Self::Info => "INFO",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Ok => "hyprbinds-health-ok",
            Self::Warn => "hyprbinds-health-warn",
            Self::Fail => "hyprbinds-health-fail",
            Self::Info => "hyprbinds-health-info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub category: &'static str,
    pub title: String,
    pub detail: String,
    pub severity: Severity,
    /// Short guidance shown under the detail.
    pub fix_hint: Option<String>,
    /// Optional shell snippet the user can copy.
    pub fix_command: Option<String>,
}

/// Run the full diagnostic suite (best-effort; missing tools become WARN/FAIL rows).
pub fn run_all() -> Vec<CheckResult> {
    let mut out = Vec::with_capacity(28);
    check_session(&mut out);
    check_hyprland(&mut out);
    check_portals(&mut out);
    check_audio(&mut out);
    check_waybar(&mut out);
    check_starship(&mut out);
    check_gpu_hints(&mut out);
    out
}

fn check_session(out: &mut Vec<CheckResult>) {
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    if wayland.is_empty() {
        out.push(CheckResult {
            category: "Session",
            title: "Wayland display".into(),
            detail: "WAYLAND_DISPLAY is unset — not running under Wayland.".into(),
            severity: Severity::Fail,
            fix_hint: Some(
                "Log into a Hyprland (Wayland) session. Screenshare and many portal features need Wayland."
                    .into(),
            ),
            fix_command: None,
        });
    } else {
        out.push(CheckResult {
            category: "Session",
            title: "Wayland display".into(),
            detail: format!("WAYLAND_DISPLAY={wayland}"),
            severity: Severity::Ok,
            fix_hint: None,
            fix_command: None,
        });
    }

    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktop_ok = desktop.to_ascii_lowercase().contains("hyprland");
    out.push(CheckResult {
        category: "Session",
        title: "Desktop identity".into(),
        detail: if desktop.is_empty() {
            "XDG_CURRENT_DESKTOP is unset.".into()
        } else {
            format!("XDG_CURRENT_DESKTOP={desktop}")
        },
        severity: if desktop_ok {
            Severity::Ok
        } else if desktop.is_empty() {
            Severity::Warn
        } else {
            Severity::Warn
        },
        fix_hint: if desktop_ok {
            None
        } else {
            Some(
                "Portals pick backends from the desktop name. Hyprland sessions should set XDG_CURRENT_DESKTOP=Hyprland."
                    .into(),
            )
        },
        fix_command: if desktop_ok {
            None
        } else {
            Some("export XDG_CURRENT_DESKTOP=Hyprland".into())
        },
    });

    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    out.push(CheckResult {
        category: "Session",
        title: "XDG runtime dir".into(),
        detail: if runtime.is_empty() {
            "XDG_RUNTIME_DIR is unset.".into()
        } else {
            format!("XDG_RUNTIME_DIR={runtime}")
        },
        severity: if runtime.is_empty() {
            Severity::Fail
        } else {
            Severity::Ok
        },
        fix_hint: if runtime.is_empty() {
            Some("Start from a proper login session (greetd/sddm/etc.) so systemd user session sets this.".into())
        } else {
            None
        },
        fix_command: None,
    });
}

fn check_hyprland(out: &mut Vec<CheckResult>) {
    if !command_exists("hyprctl") {
        out.push(CheckResult {
            category: "Hyprland",
            title: "hyprctl".into(),
            detail: "hyprctl not found on PATH.".into(),
            severity: Severity::Fail,
            fix_hint: Some("Install Hyprland so hyprctl is available.".into()),
            fix_command: Some("sudo pacman -S hyprland".into()),
        });
        return;
    }

    match run_capture("hyprctl", &["version"]) {
        Ok(text) => {
            let first = text.lines().next().unwrap_or(text.trim()).trim().to_string();
            out.push(CheckResult {
                category: "Hyprland",
                title: "hyprctl".into(),
                detail: first,
                severity: Severity::Ok,
                fix_hint: None,
                fix_command: None,
            });
        }
        Err(err) => {
            out.push(CheckResult {
                category: "Hyprland",
                title: "hyprctl".into(),
                detail: format!("hyprctl failed: {err}"),
                severity: Severity::Fail,
                fix_hint: Some(
                    "Hyprland may not be running, or this process cannot talk to the compositor socket."
                        .into(),
                ),
                fix_command: None,
            });
        }
    }

    match run_capture("hyprctl", &["monitors", "-j"]) {
        Ok(raw) => {
            let count = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v.as_array().map(|a| a.len()))
                .unwrap_or(0);
            out.push(CheckResult {
                category: "Hyprland",
                title: "Monitors".into(),
                detail: format!("{count} monitor(s) reported by hyprctl"),
                severity: if count == 0 {
                    Severity::Warn
                } else {
                    Severity::Ok
                },
                fix_hint: None,
                fix_command: None,
            });
        }
        Err(err) => {
            out.push(CheckResult {
                category: "Hyprland",
                title: "Monitors".into(),
                detail: format!("Could not query monitors: {err}"),
                severity: Severity::Warn,
                fix_hint: None,
                fix_command: None,
            });
        }
    }
}

fn check_portals(out: &mut Vec<CheckResult>) {
    let portal_bin = path_exists("/usr/lib/xdg-desktop-portal")
        || command_exists("xdg-desktop-portal");
    let hypr_portal = path_exists("/usr/lib/xdg-desktop-portal-hyprland")
        || package_file_hint("xdg-desktop-portal-hyprland");

    out.push(CheckResult {
        category: "Screenshare",
        title: "xdg-desktop-portal".into(),
        detail: if portal_bin {
            "Portal binary present.".into()
        } else {
            "xdg-desktop-portal not found.".into()
        },
        severity: if portal_bin {
            Severity::Ok
        } else {
            Severity::Fail
        },
        fix_hint: if portal_bin {
            None
        } else {
            Some("Required for file pickers, screenshare, and many Flatpak/Electron features.".into())
        },
        fix_command: if portal_bin {
            None
        } else {
            Some("sudo pacman -S xdg-desktop-portal".into())
        },
    });

    out.push(CheckResult {
        category: "Screenshare",
        title: "xdg-desktop-portal-hyprland".into(),
        detail: if hypr_portal {
            "Hyprland portal backend present.".into()
        } else {
            "Hyprland portal backend not found.".into()
        },
        severity: if hypr_portal {
            Severity::Ok
        } else {
            Severity::Fail
        },
        fix_hint: if hypr_portal {
            None
        } else {
            Some(
                "Without this backend, Chromium/Electron/Firefox screenshare often fails on Hyprland."
                    .into(),
            )
        },
        fix_command: if hypr_portal {
            None
        } else {
            Some("sudo pacman -S xdg-desktop-portal-hyprland".into())
        },
    });

    let portal_active = user_unit_active("xdg-desktop-portal.service")
        || process_running("xdg-desktop-portal");
    out.push(CheckResult {
        category: "Screenshare",
        title: "Portal service".into(),
        detail: if portal_active {
            "xdg-desktop-portal is running.".into()
        } else {
            "xdg-desktop-portal does not appear to be running.".into()
        },
        severity: if portal_active {
            Severity::Ok
        } else {
            Severity::Fail
        },
        fix_hint: if portal_active {
            None
        } else {
            Some("Restart the user portal stack after installing backends.".into())
        },
        fix_command: if portal_active {
            None
        } else {
            Some(
                "systemctl --user restart xdg-desktop-portal xdg-desktop-portal-hyprland".into(),
            )
        },
    });

    let hypr_portal_active = user_unit_active("xdg-desktop-portal-hyprland.service")
        || process_running("xdg-desktop-portal-hyprland");
    out.push(CheckResult {
        category: "Screenshare",
        title: "Hyprland portal service".into(),
        detail: if hypr_portal_active {
            "xdg-desktop-portal-hyprland is running.".into()
        } else if hypr_portal {
            "Backend installed but not running (may start on demand).".into()
        } else {
            "Hyprland portal service not running.".into()
        },
        severity: if hypr_portal_active {
            Severity::Ok
        } else if hypr_portal {
            Severity::Warn
        } else {
            Severity::Fail
        },
        fix_hint: if hypr_portal_active {
            None
        } else {
            Some(
                "If screenshare still fails, restart portals and relaunch the browser/app.".into(),
            )
        },
        fix_command: if hypr_portal_active {
            None
        } else {
            Some(
                "systemctl --user restart xdg-desktop-portal-hyprland xdg-desktop-portal".into(),
            )
        },
    });

    match portal_preference_status() {
        PortalPref::Ok(path, summary) => out.push(CheckResult {
            category: "Screenshare",
            title: "Portal preference".into(),
            detail: format!("{} ({})", summary, path.display()),
            severity: Severity::Ok,
            fix_hint: None,
            fix_command: None,
        }),
        PortalPref::Missing => out.push(CheckResult {
            category: "Screenshare",
            title: "Portal preference".into(),
            detail: "No portals.conf / hyprland-portals.conf found in ~/.config/xdg-desktop-portal."
                .into(),
            severity: Severity::Warn,
            fix_hint: Some(
                "Create a preference file so portals prefer the Hyprland backend (then gtk)."
                    .into(),
            ),
            fix_command: Some(
                "mkdir -p ~/.config/xdg-desktop-portal && printf '%s\\n' '[preferred]' 'default=hyprland;gtk' > ~/.config/xdg-desktop-portal/hyprland-portals.conf && systemctl --user restart xdg-desktop-portal".into(),
            ),
        }),
        PortalPref::Weak(path, summary) => out.push(CheckResult {
            category: "Screenshare",
            title: "Portal preference".into(),
            detail: format!("{} ({})", summary, path.display()),
            severity: Severity::Warn,
            fix_hint: Some(
                "Preference file exists but does not clearly prefer hyprland. Consider default=hyprland;gtk."
                    .into(),
            ),
            fix_command: Some(
                "printf '%s\\n' '[preferred]' 'default=hyprland;gtk' > ~/.config/xdg-desktop-portal/hyprland-portals.conf".into(),
            ),
        }),
    }

    // gtk portal is a useful fallback for file choosers
    let gtk_portal = path_exists("/usr/lib/xdg-desktop-portal-gtk");
    out.push(CheckResult {
        category: "Screenshare",
        title: "xdg-desktop-portal-gtk".into(),
        detail: if gtk_portal {
            "GTK portal fallback present (good for file dialogs).".into()
        } else {
            "GTK portal not found (optional but recommended).".into()
        },
        severity: if gtk_portal {
            Severity::Ok
        } else {
            Severity::Info
        },
        fix_hint: if gtk_portal {
            None
        } else {
            Some("Install for better file picker support in many apps.".into())
        },
        fix_command: if gtk_portal {
            None
        } else {
            Some("sudo pacman -S xdg-desktop-portal-gtk".into())
        },
    });
}

fn check_audio(out: &mut Vec<CheckResult>) {
    let pw_bin = command_exists("pipewire") || command_exists("pw-cli");
    out.push(CheckResult {
        category: "Audio",
        title: "PipeWire tools".into(),
        detail: if pw_bin {
            "PipeWire CLI tools found.".into()
        } else {
            "pipewire / pw-cli not found.".into()
        },
        severity: if pw_bin {
            Severity::Ok
        } else {
            Severity::Fail
        },
        fix_hint: if pw_bin {
            None
        } else {
            Some("Modern Hyprland desktops use PipeWire for audio and screenshare audio.".into())
        },
        fix_command: if pw_bin {
            None
        } else {
            Some("sudo pacman -S pipewire wireplumber pipewire-pulse pipewire-alsa pipewire-jack".into())
        },
    });

    let pw_active = user_unit_active("pipewire.service") || process_running("pipewire");
    out.push(CheckResult {
        category: "Audio",
        title: "PipeWire service".into(),
        detail: if pw_active {
            "pipewire is running.".into()
        } else {
            "pipewire does not appear to be running.".into()
        },
        severity: if pw_active {
            Severity::Ok
        } else {
            Severity::Fail
        },
        fix_hint: if pw_active {
            None
        } else {
            Some("Enable and start the user PipeWire units.".into())
        },
        fix_command: if pw_active {
            None
        } else {
            Some(
                "systemctl --user enable --now pipewire.service pipewire-pulse.service wireplumber.service"
                    .into(),
            )
        },
    });

    let wp_active = user_unit_active("wireplumber.service") || process_running("wireplumber");
    out.push(CheckResult {
        category: "Audio",
        title: "WirePlumber".into(),
        detail: if wp_active {
            "wireplumber is running.".into()
        } else {
            "wireplumber does not appear to be running.".into()
        },
        severity: if wp_active {
            Severity::Ok
        } else {
            Severity::Fail
        },
        fix_hint: if wp_active {
            None
        } else {
            Some("Session manager is required for devices and routing.".into())
        },
        fix_command: if wp_active {
            None
        } else {
            Some("systemctl --user enable --now wireplumber.service".into())
        },
    });

    if command_exists("pactl") {
        match run_capture("pactl", &["info"]) {
            Ok(info) => {
                let server = info
                    .lines()
                    .find(|l| l.to_ascii_lowercase().contains("server name"))
                    .unwrap_or("pactl info OK")
                    .trim()
                    .to_string();
                let sink = run_capture("pactl", &["get-default-sink"])
                    .unwrap_or_else(|_| "(unknown sink)".into())
                    .trim()
                    .to_string();
                let source = run_capture("pactl", &["get-default-source"])
                    .unwrap_or_else(|_| "(unknown source)".into())
                    .trim()
                    .to_string();
                let pulse_on_pw = server.to_ascii_lowercase().contains("pipewire");
                out.push(CheckResult {
                    category: "Audio",
                    title: "Default devices".into(),
                    detail: format!("{server} · sink={sink} · source={source}"),
                    severity: if pulse_on_pw {
                        Severity::Ok
                    } else {
                        Severity::Warn
                    },
                    fix_hint: if pulse_on_pw {
                        None
                    } else {
                        Some(
                            "pactl is not talking to PipeWire. Install pipewire-pulse and disable PulseAudio."
                                .into(),
                        )
                    },
                    fix_command: if pulse_on_pw {
                        None
                    } else {
                        Some("sudo pacman -S pipewire-pulse && systemctl --user enable --now pipewire-pulse.service".into())
                    },
                });
            }
            Err(err) => out.push(CheckResult {
                category: "Audio",
                title: "Pulse/PipeWire bridge".into(),
                detail: format!("pactl failed: {err}"),
                severity: Severity::Warn,
                fix_hint: Some("Install pipewire-pulse or start pipewire-pulse.service.".into()),
                fix_command: Some(
                    "systemctl --user enable --now pipewire-pulse.service".into(),
                ),
            }),
        }
    } else {
        out.push(CheckResult {
            category: "Audio",
            title: "pactl".into(),
            detail: "pactl not found (optional helper for sinks/sources).".into(),
            severity: Severity::Info,
            fix_hint: Some("Comes with libpulse / pipewire-pulse on most distros.".into()),
            fix_command: Some("sudo pacman -S libpulse".into()),
        });
    }
}

fn check_waybar(out: &mut Vec<CheckResult>) {
    let installed = command_exists("waybar");
    let running = crate::experimental::waybar::is_running();
    let stale = crate::experimental::waybar::has_stale_zombies();

    let config_dir = dirs::config_dir().map(|mut p| {
        p.push("waybar");
        p
    });
    let has_config = config_dir.as_ref().is_some_and(|d| {
        d.join("config.jsonc").is_file()
            || d.join("config").is_file()
            || d.join("config.json").is_file()
    });

    if !installed {
        out.push(CheckResult {
            category: "Waybar",
            title: "waybar binary".into(),
            detail: "waybar not found on PATH.".into(),
            severity: Severity::Warn,
            fix_hint: Some(
                "Install waybar to use the Waybar Studio page (optional for Hyprland alone)."
                    .into(),
            ),
            fix_command: Some("sudo pacman -S waybar".into()),
        });
    } else {
        out.push(CheckResult {
            category: "Waybar",
            title: "waybar binary".into(),
            detail: "waybar is installed.".into(),
            severity: Severity::Ok,
            fix_hint: None,
            fix_command: None,
        });
        out.push(CheckResult {
            category: "Waybar",
            title: "waybar process".into(),
            detail: if running {
                format!("Waybar is running ({})", crate::experimental::waybar::status_label())
            } else if stale {
                "Only a defunct/zombie waybar process remains — Start will ignore it and launch a new one.".into()
            } else {
                "Waybar is installed but not running.".into()
            },
            severity: if running {
                Severity::Ok
            } else if stale {
                Severity::Warn
            } else {
                Severity::Info
            },
            fix_hint: if running {
                None
            } else {
                Some("Use Waybar Studio → Start, or add `waybar` to Hyprland Startup.".into())
            },
            fix_command: if running {
                None
            } else {
                Some("setsid -f waybar".into())
            },
        });
    }

    out.push(CheckResult {
        category: "Waybar",
        title: "Waybar config".into(),
        detail: match &config_dir {
            Some(d) if has_config => format!("Found config under {}", d.display()),
            Some(d) => format!("No config/config.jsonc under {}", d.display()),
            None => "Could not resolve ~/.config/waybar".into(),
        },
        severity: if has_config {
            Severity::Ok
        } else if installed {
            Severity::Warn
        } else {
            Severity::Info
        },
        fix_hint: if has_config {
            None
        } else {
            Some("Waybar Studio can create a minimal template on first Apply.".into())
        },
        fix_command: None,
    });
}

fn check_starship(out: &mut Vec<CheckResult>) {
    let installed = command_exists("starship");
    let config = crate::experimental::starship::config_path();
    let has_config = config.as_ref().is_some_and(|p| p.is_file());
    let shells = crate::experimental::starship::detect_shells();
    let enabled = shells.iter().filter(|s| s.enabled).count();
    let installed_shells = shells.iter().filter(|s| s.installed).count();

    if !installed {
        out.push(CheckResult {
            category: "Starship",
            title: "starship binary".into(),
            detail: "starship not found on PATH.".into(),
            severity: Severity::Warn,
            fix_hint: Some(
                "Install starship to use the Starship Studio page (optional for Hyprland alone)."
                    .into(),
            ),
            fix_command: Some(crate::experimental::starship::install_hint().into()),
        });
    } else {
        out.push(CheckResult {
            category: "Starship",
            title: "starship binary".into(),
            detail: format!("starship is installed — {}", crate::experimental::starship::status_label()),
            severity: Severity::Ok,
            fix_hint: None,
            fix_command: None,
        });
    }

    out.push(CheckResult {
        category: "Starship",
        title: "starship.toml".into(),
        detail: match &config {
            Some(p) if has_config => format!("Found {}", p.display()),
            Some(p) => format!("No config yet at {}", p.display()),
            None => "Could not resolve starship.toml path".into(),
        },
        severity: if has_config {
            Severity::Ok
        } else if installed {
            Severity::Info
        } else {
            Severity::Info
        },
        fix_hint: if has_config {
            None
        } else {
            Some("Starship Studio can create a starter starship.toml on Apply.".into())
        },
        fix_command: None,
    });

    out.push(CheckResult {
        category: "Starship",
        title: "Shell integration".into(),
        detail: format!(
            "{enabled}/{installed_shells} installed shell(s) have Starship init (detected: {})",
            if shells.is_empty() {
                "none".into()
            } else {
                shells
                    .iter()
                    .map(|s| {
                        format!(
                            "{}{}",
                            s.kind.id(),
                            if s.enabled { "*" } else { "" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ),
        severity: if enabled > 0 {
            Severity::Ok
        } else if installed && installed_shells > 0 {
            Severity::Info
        } else {
            Severity::Info
        },
        fix_hint: if enabled > 0 {
            None
        } else {
            Some("Use Starship Studio → Shells → Enable for zsh/fish/bash/…".into())
        },
        fix_command: None,
    });
}

fn check_gpu_hints(out: &mut Vec<CheckResult>) {
    let nvidia = path_exists("/proc/driver/nvidia/version")
        || path_exists("/dev/nvidia0")
        || command_exists("nvidia-smi");
    if nvidia {
        let gbm = std::env::var("GBM_BACKEND").unwrap_or_default();
        let nvidia_wayland = std::env::var("__GLX_VENDOR_LIBRARY_NAME").unwrap_or_default();
        out.push(CheckResult {
            category: "GPU",
            title: "NVIDIA detected".into(),
            detail: format!(
                "NVIDIA stack present. GBM_BACKEND={gbm:?} __GLX_VENDOR_LIBRARY_NAME={nvidia_wayland:?}"
            ),
            severity: Severity::Info,
            fix_hint: Some(
                "Screenshare/black windows on NVIDIA often need up-to-date drivers plus Hyprland nvidia env hints. Prefer explicit sync-capable drivers."
                    .into(),
            ),
            fix_command: Some(
                "echo 'See Hyprland NVIDIA wiki — avoid outdated LIBVA/GBM hacks unless you know you need them.'"
                    .into(),
            ),
        });
    } else {
        out.push(CheckResult {
            category: "GPU",
            title: "GPU".into(),
            detail: "No NVIDIA driver nodes detected (fine for AMD/Intel).".into(),
            severity: Severity::Ok,
            fix_hint: None,
            fix_command: None,
        });
    }

    let gdk = std::env::var("GDK_BACKEND").unwrap_or_default();
    if !gdk.is_empty() && gdk != "wayland" && !gdk.contains("wayland") {
        out.push(CheckResult {
            category: "GPU",
            title: "GDK_BACKEND".into(),
            detail: format!("GDK_BACKEND={gdk}"),
            severity: Severity::Warn,
            fix_hint: Some(
                "Forcing x11 can break Wayland portals for GTK apps. Prefer unset or wayland."
                    .into(),
            ),
            fix_command: Some("unset GDK_BACKEND".into()),
        });
    }
}

enum PortalPref {
    Ok(PathBuf, String),
    Weak(PathBuf, String),
    Missing,
}

fn portal_preference_status() -> PortalPref {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = home.join(".config/xdg-desktop-portal");
    let candidates = [
        dir.join("hyprland-portals.conf"),
        dir.join("portals.conf"),
    ];
    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(&path) {
            let lower = text.to_ascii_lowercase();
            let mentions_hypr = lower.contains("hyprland");
            let summary = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .take(3)
                .collect::<Vec<_>>()
                .join(" · ");
            let summary = if summary.is_empty() {
                "empty file".into()
            } else {
                summary
            };
            return if mentions_hypr {
                PortalPref::Ok(path, summary)
            } else {
                PortalPref::Weak(path, summary)
            };
        }
    }
    PortalPref::Missing
}

fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

fn package_file_hint(name: &str) -> bool {
    Path::new(&format!("/usr/share/xdg-desktop-portal/portals/{name}.portal")).exists()
        || (name.contains("hyprland")
            && Path::new("/usr/share/xdg-desktop-portal/portals/hyprland.portal").exists())
}

fn user_unit_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", unit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn process_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .status()
        .map(|s| s.success())
        .unwrap_or_else(|_| {
            Command::new("pgrep")
                .arg(name)
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        })
}

fn run_capture(bin: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let err = err.trim();
        if err.is_empty() {
            return Err(format!("{bin} exited with {}", output.status));
        }
        return Err(err.to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Summary counts for the toolbar / status line.
pub fn summarize(results: &[CheckResult]) -> (usize, usize, usize) {
    let mut ok = 0;
    let mut warn = 0;
    let mut fail = 0;
    for r in results {
        match r.severity {
            Severity::Ok => ok += 1,
            Severity::Warn => warn += 1,
            Severity::Fail => fail += 1,
            Severity::Info => {}
        }
    }
    (ok, warn, fail)
}
