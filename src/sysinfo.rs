//! Collect pasteable system info for debugging (hardware + key software versions).

use std::path::Path;
use std::process::Command;

/// Build a multi-section report safe to paste in Discord/GitHub.
/// Omits serial numbers / UUIDs by default.
pub fn collect_report() -> String {
    let mut out = String::with_capacity(8_192);
    out.push_str("=== Hyprbinds system info ===\n");
    out.push_str(&format!("Generated: {}\n\n", now_stamp()));

    section(&mut out, "OS / Kernel", &os_block());
    section(&mut out, "Motherboard / BIOS", &dmi_block());
    section(&mut out, "CPU", &cpu_block());
    section(&mut out, "Memory", &memory_block());
    section(&mut out, "GPU / Display PCI", &gpu_block());
    section(&mut out, "PCI devices", &pci_block());
    section(&mut out, "USB devices", &usb_block());
    section(&mut out, "Block devices", &block_block());
    section(&mut out, "Audio", &audio_block());
    section(&mut out, "Session / Hyprland", &session_block());
    section(&mut out, "Portals / PipeWire versions", &stack_versions_block());
    section(&mut out, "Environment (selected)", &env_block());

    out.push_str("\n(Serial numbers and product UUIDs omitted.)\n");
    out
}

fn section(out: &mut String, title: &str, body: &str) {
    out.push_str("## ");
    out.push_str(title);
    out.push('\n');
    let body = body.trim();
    if body.is_empty() {
        out.push_str("(unavailable)\n\n");
    } else {
        out.push_str(body);
        out.push_str("\n\n");
    }
}

fn now_stamp() -> String {
    // Local-ish stamp without extra deps: use `date` if present.
    run_ok("date", &["--iso-8601=seconds"]).unwrap_or_else(|| "unknown".into())
}

fn os_block() -> String {
    let mut lines = Vec::new();
    if let Some(pretty) = read_os_release_pretty() {
        lines.push(format!("distro: {pretty}"));
    }
    if let Some(u) = run_ok("uname", &["-a"]) {
        lines.push(format!("uname: {u}"));
    }
    if let Some(h) = run_ok("hostnamectl", &["--status"]) {
        // Keep the useful hostnamectl lines only.
        for line in h.lines() {
            let t = line.trim();
            if t.starts_with("Operating System:")
                || t.starts_with("Kernel:")
                || t.starts_with("Architecture:")
                || t.starts_with("Hardware Vendor:")
                || t.starts_with("Hardware Model:")
                || t.starts_with("Firmware Version:")
            {
                lines.push(t.to_string());
            }
        }
    }
    lines.join("\n")
}

fn dmi_block() -> String {
    let keys = [
        ("sys_vendor", "vendor"),
        ("product_name", "product"),
        ("product_version", "product_version"),
        ("product_family", "family"),
        ("product_sku", "sku"),
        ("board_vendor", "board_vendor"),
        ("board_name", "board"),
        ("board_version", "board_version"),
        ("bios_vendor", "bios_vendor"),
        ("bios_version", "bios_version"),
        ("bios_date", "bios_date"),
        ("chassis_type", "chassis_type"),
        ("chassis_vendor", "chassis_vendor"),
    ];
    let mut lines = Vec::new();
    for (file, label) in keys {
        if let Some(v) = read_dmi(file) {
            if !is_placeholder(&v) {
                lines.push(format!("{label}: {v}"));
            }
        }
    }
    lines.join("\n")
}

fn cpu_block() -> String {
    if let Some(lscpu) = run_ok("lscpu", &[]) {
        let want = [
            "Architecture:",
            "CPU(s):",
            "Model name:",
            "Vendor ID:",
            "Thread(s) per core:",
            "Core(s) per socket:",
            "Socket(s):",
            "CPU max MHz:",
            "CPU min MHz:",
            "Flags:",
            "Vulnerability",
        ];
        let mut lines: Vec<String> = lscpu
            .lines()
            .filter(|l| want.iter().any(|w| l.trim_start().starts_with(w)))
            .map(|l| l.trim().to_string())
            .collect();
        // Flags line can be huge — keep a short note instead of full dump.
        lines = lines
            .into_iter()
            .map(|l| {
                if l.starts_with("Flags:") {
                    let count = l.split_whitespace().count().saturating_sub(1);
                    format!("Flags: ({count} flags; omitted for size)")
                } else {
                    l
                }
            })
            .collect();
        if !lines.is_empty() {
            return lines.join("\n");
        }
    }
    // Fallback /proc/cpuinfo
    let mut model = None;
    let mut cores = 0usize;
    if let Ok(text) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("model name") {
                model = Some(rest.trim_start_matches(':').trim().to_string());
            }
            if line.starts_with("processor") {
                cores += 1;
            }
        }
    }
    match model {
        Some(m) => format!("model: {m}\nlogical_cpus: {cores}"),
        None => String::new(),
    }
}

fn memory_block() -> String {
    let mut lines = Vec::new();
    if let Some(free) = run_ok("free", &["-h"]) {
        lines.push(free.trim().to_string());
    }
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        for key in ["MemTotal:", "MemAvailable:", "SwapTotal:", "SwapFree:"] {
            if let Some(line) = meminfo.lines().find(|l| l.starts_with(key)) {
                lines.push(line.trim().to_string());
            }
        }
    }
    lines.join("\n")
}

fn gpu_block() -> String {
    let mut lines = Vec::new();
    if let Some(pci) = run_ok("lspci", &[]) {
        for line in pci.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("vga")
                || lower.contains("3d")
                || lower.contains("display controller")
            {
                lines.push(line.trim().to_string());
            }
        }
    }
    if command_exists("nvidia-smi") {
        if let Some(smi) = run_ok(
            "nvidia-smi",
            &[
                "--query-gpu=name,driver_version,memory.total",
                "--format=csv,noheader",
            ],
        ) {
            lines.push(format!("nvidia-smi: {}", smi.trim()));
        }
    }
    // DRM cards
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        let mut cards = Vec::new();
        for ent in entries.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if name.starts_with("card") && !name.contains('-') {
                let vendor = read_to_string(ent.path().join("device/vendor"));
                let device = read_to_string(ent.path().join("device/device"));
                cards.push(format!(
                    "{name}: vendor={} device={}",
                    vendor.unwrap_or_else(|| "?".into()),
                    device.unwrap_or_else(|| "?".into())
                ));
            }
        }
        if !cards.is_empty() {
            lines.push(format!("DRM: {}", cards.join(" · ")));
        }
    }
    lines.join("\n")
}

fn pci_block() -> String {
    run_ok("lspci", &["-nn"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn usb_block() -> String {
    run_ok("lsusb", &[])
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            run_ok("lsusb", &["-v"])
                .map(|_| "(lsusb produced no short listing)".into())
                .unwrap_or_default()
        })
}

fn block_block() -> String {
    // No serials: NAME, SIZE, TYPE, MODEL, TRAN, FSTYPE, MOUNTPOINTS
    run_ok(
        "lsblk",
        &[
            "-o",
            "NAME,SIZE,TYPE,MODEL,TRAN,FSTYPE,MOUNTPOINTS",
            "-e",
            "7",
        ],
    )
    .map(|s| s.trim().to_string())
    .unwrap_or_default()
}

fn audio_block() -> String {
    let mut lines = Vec::new();
    if let Some(info) = run_ok("pactl", &["info"]) {
        for key in [
            "Server Name:",
            "Server Version:",
            "Default Sink:",
            "Default Source:",
        ] {
            if let Some(line) = info.lines().find(|l| l.trim_start().starts_with(key)) {
                lines.push(line.trim().to_string());
            }
        }
    }
    if let Some(sinks) = run_ok("pactl", &["list", "short", "sinks"]) {
        lines.push("sinks:".into());
        lines.push(sinks.trim().to_string());
    }
    if let Some(sources) = run_ok("pactl", &["list", "short", "sources"]) {
        lines.push("sources:".into());
        lines.push(sources.trim().to_string());
    }
    lines.join("\n")
}

fn session_block() -> String {
    let mut lines = Vec::new();
    for key in [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_TYPE",
        "XDG_SESSION_DESKTOP",
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XDG_RUNTIME_DIR",
    ] {
        if let Ok(v) = std::env::var(key) {
            lines.push(format!("{key}={v}"));
        }
    }
    if let Some(v) = run_ok("hyprctl", &["version"]) {
        let first = v.lines().next().unwrap_or(v.trim()).trim();
        lines.push(format!("hyprctl: {first}"));
    }
    if let Some(m) = run_ok("hyprctl", &["monitors"]) {
        lines.push("monitors:".into());
        // Keep a compact subset of monitor lines; drop serials.
        for line in m.lines().take(60) {
            let t = line.trim_end();
            if t.trim_start().starts_with("serial:") {
                continue;
            }
            if !t.is_empty() {
                lines.push(t.to_string());
            }
        }
    }
    lines.join("\n")
}

fn stack_versions_block() -> String {
    let mut lines = Vec::new();
    push_version(&mut lines, "pipewire", &["--version"]);
    push_version(&mut lines, "pw-cli", &["--version"]);
    // wireplumber often uses --version
    push_version(&mut lines, "wireplumber", &["--version"]);
    if let Some(v) = package_version("xdg-desktop-portal") {
        lines.push(format!("xdg-desktop-portal: {v}"));
    }
    if let Some(v) = package_version("xdg-desktop-portal-hyprland") {
        lines.push(format!("xdg-desktop-portal-hyprland: {v}"));
    }
    if let Some(v) = package_version("xdg-desktop-portal-gtk") {
        lines.push(format!("xdg-desktop-portal-gtk: {v}"));
    }
    if let Some(v) = package_version("hyprland") {
        lines.push(format!("hyprland pkg: {v}"));
    }
    if let Some(v) = package_version("mesa") {
        lines.push(format!("mesa: {v}"));
    }
    if let Some(v) = package_version("linux") {
        lines.push(format!("linux pkg: {v}"));
    } else if let Some(v) = package_version("linux-cachyos") {
        lines.push(format!("linux-cachyos: {v}"));
    }
    // GTK / app
    lines.push(format!(
        "hyprbinds: {}",
        env!("CARGO_PKG_VERSION")
    ));
    lines.join("\n")
}

fn env_block() -> String {
    let keys = [
        "GDK_BACKEND",
        "QT_QPA_PLATFORM",
        "SDL_VIDEODRIVER",
        "WLR_NO_HARDWARE_CURSORS",
        "GBM_BACKEND",
        "__GLX_VENDOR_LIBRARY_NAME",
        "LIBVA_DRIVER_NAME",
        "AQ_DRM_DEVICES",
        "XCURSOR_SIZE",
        "HYPRLAND_INSTANCE_SIGNATURE",
    ];
    let mut lines = Vec::new();
    for key in keys {
        match std::env::var(key) {
            Ok(v) if !v.is_empty() => lines.push(format!("{key}={v}")),
            _ => {}
        }
    }
    if lines.is_empty() {
        "(none of the common override vars are set)".into()
    } else {
        lines.join("\n")
    }
}

fn push_version(lines: &mut Vec<String>, bin: &str, args: &[&str]) {
    if let Some(v) = run_ok(bin, args) {
        let first = v.lines().next().unwrap_or(v.trim()).trim();
        lines.push(format!("{bin}: {first}"));
    }
}

fn package_version(name: &str) -> Option<String> {
    // Arch/CachyOS
    if let Some(o) = run_ok("pacman", &["-Q", name]) {
        return Some(o.trim().to_string());
    }
    // Debian/Ubuntu
    if let Some(o) = run_ok("dpkg-query", &["-W", "-f=${Package} ${Version}\n", name]) {
        let t = o.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    // Fedora
    if let Some(o) = run_ok("rpm", &["-q", name]) {
        let t = o.trim();
        if !t.contains("not installed") {
            return Some(t.to_string());
        }
    }
    None
}

fn read_dmi(name: &str) -> Option<String> {
    read_to_string(Path::new("/sys/class/dmi/id").join(name))
}

fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn is_placeholder(v: &str) -> bool {
    let u = v.to_ascii_uppercase();
    u.contains("TO BE FILLED")
        || u.contains("DEFAULT STRING")
        || u == "NONE"
        || u == "NOT SPECIFIED"
        || u == "SYSTEM PRODUCT NAME"
        || u == "SYSTEM VERSION"
        || u == "SYSTEM MANUFACTURER"
}

fn read_os_release_pretty() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

fn command_exists(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin} >/dev/null 2>&1")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_ok(bin: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(bin).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).to_string();
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}
