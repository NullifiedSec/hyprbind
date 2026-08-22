//! Audio panel (experimental).

mod ui;

pub use ui::*;

use std::process::Command;

#[derive(Debug, Clone)]
pub struct SinkInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub id: u32,
    pub name: String,
    pub sink: String,
    pub volume_pct: u8,
    pub muted: bool,
}

#[derive(Debug, Clone)]
pub struct AlsaControl {
    pub name: String,
    pub volume_pct: Option<u8>,
    pub muted: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub default_sink: String,
    pub default_source: String,
    pub sink_volume_pct: u8,
    pub sink_muted: bool,
    pub source_volume_pct: u8,
    pub source_muted: bool,
    pub sinks: Vec<SinkInfo>,
    pub sources: Vec<SinkInfo>,
    pub streams: Vec<StreamInfo>,
    pub alsa: Vec<AlsaControl>,
    pub backend: String,
}

pub fn snapshot() -> Result<AudioSnapshot, String> {
    let default_sink = run_trim("pactl", &["get-default-sink"]).unwrap_or_default();
    let default_source = run_trim("pactl", &["get-default-source"]).unwrap_or_default();

    let (sink_volume_pct, sink_muted) = volume_of("@DEFAULT_AUDIO_SINK@")
        .or_else(|| pactl_volume("sink", &default_sink))
        .unwrap_or((50, false));
    let (source_volume_pct, source_muted) = volume_of("@DEFAULT_AUDIO_SOURCE@")
        .or_else(|| pactl_volume("source", &default_source))
        .unwrap_or((50, false));

    let sinks = list_devices("sinks");
    let sources = list_devices("sources");
    let streams = list_streams();
    let alsa = list_alsa_controls();

    let backend = run_trim("pactl", &["info"])
        .and_then(|info| {
            info.lines()
                .find(|l| l.to_ascii_lowercase().contains("server name"))
                .map(|l| l.trim().to_string())
        })
        .unwrap_or_else(|| "PipeWire / Pulse".into());

    Ok(AudioSnapshot {
        default_sink,
        default_source,
        sink_volume_pct,
        sink_muted,
        source_volume_pct,
        source_muted,
        sinks,
        sources,
        streams,
        alsa,
        backend,
    })
}

pub fn set_sink_volume(pct: u8) -> Result<(), String> {
    let pct = pct.min(150);
    if command_exists("wpctl") {
        let _ = run_ok(
            "wpctl",
            &["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{:.2}", pct as f64 / 100.0)],
        );
    }
    run_ok(
        "pactl",
        &["set-sink-volume", "@DEFAULT_SINK@", &format!("{pct}%")],
    )?;
    Ok(())
}

pub fn set_source_volume(pct: u8) -> Result<(), String> {
    let pct = pct.min(150);
    if command_exists("wpctl") {
        let _ = run_ok(
            "wpctl",
            &[
                "set-volume",
                "@DEFAULT_AUDIO_SOURCE@",
                &format!("{:.2}", pct as f64 / 100.0),
            ],
        );
    }
    run_ok(
        "pactl",
        &["set-source-volume", "@DEFAULT_SOURCE@", &format!("{pct}%")],
    )?;
    Ok(())
}

pub fn toggle_sink_mute() -> Result<(), String> {
    if command_exists("wpctl") {
        let _ = run_ok("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"]);
    }
    run_ok("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"])?;
    Ok(())
}

pub fn toggle_source_mute() -> Result<(), String> {
    if command_exists("wpctl") {
        let _ = run_ok("wpctl", &["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"]);
    }
    run_ok("pactl", &["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])?;
    Ok(())
}

pub fn set_default_sink(name: &str) -> Result<(), String> {
    run_ok("pactl", &["set-default-sink", name])?;
    Ok(())
}

pub fn set_default_source(name: &str) -> Result<(), String> {
    run_ok("pactl", &["set-default-source", name])?;
    Ok(())
}

pub fn set_stream_volume(id: u32, pct: u8) -> Result<(), String> {
    run_ok(
        "pactl",
        &["set-sink-input-volume", &id.to_string(), &format!("{}%", pct.min(150))],
    )?;
    Ok(())
}

pub fn toggle_stream_mute(id: u32) -> Result<(), String> {
    run_ok(
        "pactl",
        &["set-sink-input-mute", &id.to_string(), "toggle"],
    )?;
    Ok(())
}

pub fn set_alsa_volume(control: &str, pct: u8) -> Result<(), String> {
    run_ok(
        "amixer",
        &["-q", "sset", control, &format!("{}%", pct.min(100))],
    )?;
    Ok(())
}

pub fn toggle_alsa_mute(control: &str) -> Result<(), String> {
    run_ok("amixer", &["-q", "sset", control, "toggle"])?;
    Ok(())
}

/// Launch `alsamixer` in a terminal if possible.
pub fn open_alsamixer() -> Result<String, String> {
    let terminals = [
        ("kitty", vec!["-e", "alsamixer"]),
        ("alacritty", vec!["-e", "alsamixer"]),
        ("foot", vec!["alsamixer"]),
        ("ghostty", vec!["-e", "alsamixer"]),
        ("wezterm", vec!["start", "--", "alsamixer"]),
        ("xdg-terminal-exec", vec!["alsamixer"]),
        ("xterm", vec!["-e", "alsamixer"]),
    ];
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.is_empty() {
            let status = Command::new(&term)
                .args(["-e", "alsamixer"])
                .spawn()
                .map(|_| ())
                .or_else(|_| {
                    Command::new(&term)
                        .arg("alsamixer")
                        .spawn()
                        .map(|_| ())
                });
            if status.is_ok() {
                return Ok(format!("Opened alsamixer via {term}"));
            }
        }
    }
    for (bin, args) in terminals {
        if command_exists(bin) {
            Command::new(bin)
                .args(&args)
                .spawn()
                .map_err(|e| e.to_string())?;
            return Ok(format!("Opened alsamixer via {bin}"));
        }
    }
    Err("No terminal found to launch alsamixer (set $TERMINAL)".into())
}

fn volume_of(target: &str) -> Option<(u8, bool)> {
    let out = run_trim("wpctl", &["get-volume", target])?;
    // "Volume: 0.98" or "Volume: 0.98 [MUTED]"
    let muted = out.to_ascii_lowercase().contains("muted");
    let vol = out
        .split_whitespace()
        .nth(1)?
        .parse::<f64>()
        .ok()?;
    Some(((vol * 100.0).round().clamp(0.0, 150.0) as u8, muted))
}

fn pactl_volume(kind: &str, name: &str) -> Option<(u8, bool)> {
    if name.is_empty() {
        return None;
    }
    let args = if kind == "sink" {
        vec!["get-sink-volume", name]
    } else {
        vec!["get-source-volume", name]
    };
    let out = run_trim("pactl", &args)?;
    // Volume: front-left: 63944 /  98% / ...
    let pct = out
        .split('%')
        .next()?
        .split_whitespace()
        .rev()
        .find_map(|t| t.parse::<u8>().ok())?;
    let mute_args = if kind == "sink" {
        ["get-sink-mute", name]
    } else {
        ["get-source-mute", name]
    };
    let mute_out = run_trim("pactl", &mute_args).unwrap_or_default();
    let muted = mute_out.to_ascii_lowercase().contains("yes");
    Some((pct, muted))
}

fn list_devices(kind: &str) -> Vec<SinkInfo> {
    let Some(raw) = run_trim("pactl", &["list", "short", kind]) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let id = parts.next()?.to_string();
            let name = parts.next()?.to_string();
            Some(SinkInfo {
                id,
                description: name.clone(),
                name,
            })
        })
        .collect()
}

fn list_streams() -> Vec<StreamInfo> {
    let Some(raw) = run_trim("pactl", &["list", "sink-inputs"]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cur_id: Option<u32> = None;
    let mut name = String::new();
    let mut sink = String::new();
    let mut volume_pct = 50u8;
    let mut muted = false;

    let flush = |out: &mut Vec<StreamInfo>,
                 cur_id: &mut Option<u32>,
                 name: &mut String,
                 sink: &mut String,
                 volume_pct: &mut u8,
                 muted: &mut bool| {
        if let Some(id) = cur_id.take() {
            out.push(StreamInfo {
                id,
                name: if name.is_empty() {
                    format!("stream {id}")
                } else {
                    name.clone()
                },
                sink: sink.clone(),
                volume_pct: *volume_pct,
                muted: *muted,
            });
        }
        name.clear();
        sink.clear();
        *volume_pct = 50;
        *muted = false;
    };

    for line in raw.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Sink Input #") {
            flush(
                &mut out,
                &mut cur_id,
                &mut name,
                &mut sink,
                &mut volume_pct,
                &mut muted,
            );
            cur_id = rest.trim().parse().ok();
        } else if t.starts_with("Sink:") {
            sink = t.trim_start_matches("Sink:").trim().to_string();
        } else if t.starts_with("Mute:") {
            muted = t.to_ascii_lowercase().contains("yes");
        } else if t.starts_with("Volume:") {
            if let Some(p) = t.split('%').next().and_then(|s| {
                s.split_whitespace()
                    .rev()
                    .find_map(|x| x.parse::<u8>().ok())
            }) {
                volume_pct = p;
            }
        } else if let Some(app) = t.strip_prefix("application.name = ") {
            name = app.trim().trim_matches('"').to_string();
        } else if name.is_empty() {
            if let Some(media) = t.strip_prefix("media.name = ") {
                name = media.trim().trim_matches('"').to_string();
            }
        }
    }
    flush(
        &mut out,
        &mut cur_id,
        &mut name,
        &mut sink,
        &mut volume_pct,
        &mut muted,
    );
    out
}

fn list_alsa_controls() -> Vec<AlsaControl> {
    let Some(raw) = run_trim("amixer", &["scontrols"]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in raw.lines() {
        // Simple mixer control 'Master',0
        let Some(start) = line.find('\'') else {
            continue;
        };
        let rest = &line[start + 1..];
        let Some(end) = rest.find('\'') else {
            continue;
        };
        let name = rest[..end].to_string();
        if matches!(
            name.as_str(),
            "Master" | "PCM" | "Speaker" | "Headphone" | "Capture" | "Mic" | "Headset"
        ) {
            let (volume_pct, muted) = alsa_control_state(&name);
            out.push(AlsaControl {
                name,
                volume_pct,
                muted,
            });
        }
    }
    if out.is_empty() {
        // Fallback: try Master anyway
        let (volume_pct, muted) = alsa_control_state("Master");
        if volume_pct.is_some() || muted.is_some() {
            out.push(AlsaControl {
                name: "Master".into(),
                volume_pct,
                muted,
            });
        }
    }
    out
}

fn alsa_control_state(name: &str) -> (Option<u8>, Option<bool>) {
    let Some(raw) = run_trim("amixer", &["sget", name]) else {
        return (None, None);
    };
    let muted = if raw.contains("[off]") {
        Some(true)
    } else if raw.contains("[on]") {
        Some(false)
    } else {
        None
    };
    let pct = raw.lines().find_map(|l| {
        let l = l.trim();
        let start = l.find('[')?;
        let end = l[start + 1..].find('%')?;
        l[start + 1..start + 1 + end].parse::<u8>().ok()
    });
    (pct, muted)
}

fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_ok(bin: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(if err.trim().is_empty() {
            format!("{bin} failed")
        } else {
            err.trim().to_string()
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_trim(bin: &str, args: &[&str]) -> Option<String> {
    run_ok(bin, args)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
