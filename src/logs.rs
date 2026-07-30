//! User-session journal log collector and filter helpers (`journalctl --user`).

use serde_json::Value;
use std::process::Command;

const DEFAULT_LIMIT: usize = 400;

/// Syslog / journal priority (0 = emergency … 7 = debug).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Info,
    Debug,
}

impl Severity {
    pub fn from_priority(p: u8) -> Self {
        match p {
            0 => Self::Emergency,
            1 => Self::Alert,
            2 => Self::Critical,
            3 => Self::Error,
            4 => Self::Warning,
            5 => Self::Notice,
            6 => Self::Info,
            _ => Self::Debug,
        }
    }

    pub fn priority(self) -> u8 {
        match self {
            Self::Emergency => 0,
            Self::Alert => 1,
            Self::Critical => 2,
            Self::Error => 3,
            Self::Warning => 4,
            Self::Notice => 5,
            Self::Info => 6,
            Self::Debug => 7,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Emergency => "EMERG",
            Self::Alert => "ALERT",
            Self::Critical => "CRIT",
            Self::Error => "ERROR",
            Self::Warning => "WARN",
            Self::Notice => "NOTE",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Emergency | Self::Alert | Self::Critical | Self::Error => "hyprbinds-log-error",
            Self::Warning => "hyprbinds-log-warn",
            Self::Notice | Self::Info => "hyprbinds-log-info",
            Self::Debug => "hyprbinds-log-debug",
        }
    }
}

/// UI / filter grouping for journal severities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityFilter {
    All,
    Errors,
    Warnings,
    InfoAndUp,
    Debug,
    Specific(Severity),
}

impl SeverityFilter {
    pub fn matches(self, severity: Severity) -> bool {
        match self {
            Self::All => true,
            Self::Errors => severity.priority() <= 3,
            Self::Warnings => severity == Severity::Warning,
            Self::InfoAndUp => matches!(severity, Severity::Notice | Severity::Info),
            Self::Debug => severity == Severity::Debug,
            Self::Specific(s) => severity == s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: usize,
    pub timestamp_display: String,
    pub severity: Severity,
    pub message: String,
    pub identifier: String,
    pub unit: String,
    pub tags: Vec<String>,
    pub raw_priority: u8,
}

/// Fetch recent user-session logs for the current boot (default limit 400).
pub fn fetch(limit: usize) -> Result<Vec<LogEntry>, String> {
    fetch_with_args(limit, &[])
}

/// Fetch logs with extra `journalctl` arguments (e.g. `-u`, `--identifier=`).
pub fn fetch_with_args(limit: usize, extra_args: &[&str]) -> Result<Vec<LogEntry>, String> {
    let limit = if limit == 0 { DEFAULT_LIMIT } else { limit };
    let output = run_journalctl(limit, extra_args)?;
    Ok(parse_lines(&output))
}

/// Build extra args to filter by systemd user unit (`journalctl -u UNIT`).
pub fn unit_filter_args(unit: &str) -> Vec<String> {
    vec!["-u".into(), unit.into()]
}

/// Build extra args to filter by syslog identifier.
pub fn identifier_filter_args(identifier: &str) -> Vec<String> {
    vec![format!("--identifier={identifier}")]
}

pub fn severity_label(severity: Severity) -> &'static str {
    severity.label()
}

pub fn css_class(severity: Severity) -> &'static str {
    severity.css_class()
}

pub fn matches(
    entry: &LogEntry,
    query: &str,
    severity_filter: SeverityFilter,
    tag_filter: &str,
) -> bool {
    if !severity_filter.matches(entry.severity) {
        return false;
    }
    if !tag_filter.is_empty() && !entry.tags.iter().any(|t| t == tag_filter) {
        return false;
    }
    if query.is_empty() {
        return true;
    }
    let q = query.to_ascii_lowercase();
    entry.message.to_ascii_lowercase().contains(&q)
        || entry.identifier.to_ascii_lowercase().contains(&q)
        || entry.unit.to_ascii_lowercase().contains(&q)
        || entry.tags.iter().any(|t| t.to_ascii_lowercase().contains(&q))
}

pub fn known_tags(entries: &[LogEntry]) -> Vec<String> {
    let mut tags: Vec<String> = entries
        .iter()
        .flat_map(|e| e.tags.iter().cloned())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

fn run_journalctl(limit: usize, extra_args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("journalctl");
    cmd.args([
        "--user",
        "-b",
        "--no-pager",
        "-o",
        "json",
        "-n",
        &limit.to_string(),
    ]);
    cmd.args(extra_args);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run journalctl: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "journalctl exited with {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("journalctl output is not UTF-8: {e}"))
}

fn parse_lines(raw: &str) -> Vec<LogEntry> {
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(entry) = parse_line(line, idx) {
            out.push(entry);
        }
    }
    out
}

fn parse_line(line: &str, id: usize) -> Option<LogEntry> {
    let v: Value = serde_json::from_str(line).ok()?;

    let raw_priority = field_string(&v, "PRIORITY")
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(6);
    let severity = Severity::from_priority(raw_priority);

    let message = message_field(&v).unwrap_or_else(|| "(no message)".into());
    let identifier = first_non_empty(&v, &["SYSLOG_IDENTIFIER", "COMM", "_COMM"]);
    let unit = first_non_empty(
        &v,
        &[
            "_SYSTEMD_USER_UNIT",
            "UNIT",
            "_SYSTEMD_UNIT",
            "_SYSTEMD_SLICE",
        ],
    );

    let timestamp_display = field_string(&v, "__REALTIME_TIMESTAMP")
        .and_then(|s| format_timestamp(&s))
        .unwrap_or_else(|| "—".into());

    let mut tags = derive_tags(severity, &identifier, &unit, &message);
    if !identifier.is_empty() {
        tags.push(identifier.clone());
    }
    tags.sort();
    tags.dedup();

    Some(LogEntry {
        id,
        timestamp_display,
        severity,
        message,
        identifier,
        unit,
        tags,
        raw_priority,
    })
}

fn field_string(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|val| match val {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn message_field(v: &Value) -> Option<String> {
    let val = v.get("MESSAGE")?;
    match val {
        Value::String(s) => Some(s.clone()),
        Value::Array(arr) => {
            let bytes: Vec<u8> = arr
                .iter()
                .filter_map(|n| n.as_u64().map(|b| b as u8))
                .collect();
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
        _ => None,
    }
}

fn first_non_empty(v: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|k| {
            field_string(v, k).filter(|s| !s.is_empty())
        })
        .unwrap_or_default()
}

fn format_timestamp(micros_str: &str) -> Option<String> {
    let micros: i64 = micros_str.parse().ok()?;
    let secs = micros / 1_000_000;
    let sub_ms = (micros.rem_euclid(1_000_000) / 1_000) as u32;
    Some(format_epoch_utc(secs, sub_ms))
}

/// Compact UTC timestamp for log rows (no extra time crate).
fn format_epoch_utc(secs: i64, millis: u32) -> String {
    if secs < 0 {
        return format!("{secs}.{millis:03}");
    }
    let (year, mon, day, hour, min, sec) = utc_parts(secs as u64);
    format!(
        "{year:04}-{mon:02}-{day:02} {hour:02}:{min:02}:{sec:02}.{millis:03}"
    )
}

fn utc_parts(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hour = (rem / 3_600) as u32;
    let min = ((rem % 3_600) / 60) as u32;
    let sec = (rem % 60) as u32;

    let mut y = 1970u32;
    let mut remaining = days;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }

    let leap = is_leap(y);
    let month_lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1u32;
    for &len in &month_lengths {
        if remaining < len {
            return (y, m, remaining as u32 + 1, hour, min, sec);
        }
        remaining -= len;
        m += 1;
    }
    (y, 12, 31, hour, min, sec)
}

fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const SERVICE_HINTS: &[(&str, &str)] = &[
    ("hyprland", "hyprland"),
    ("hypr", "hyprland"),
    ("xdg-desktop-portal", "portal"),
    ("portal", "portal"),
    ("pipewire", "pipewire"),
    ("wireplumber", "wireplumber"),
    ("wlroots", "wlroots"),
    ("mutter", "mutter"),
    ("systemd", "systemd"),
    ("polkit", "polkit"),
    ("dbus", "dbus"),
    ("wayland", "wayland"),
    ("gtk", "gtk"),
    ("gdm", "gdm"),
    ("sddm", "sddm"),
];

fn derive_tags(severity: Severity, identifier: &str, unit: &str, message: &str) -> Vec<String> {
    let mut tags = Vec::new();
    match severity {
        Severity::Emergency | Severity::Alert | Severity::Critical | Severity::Error => {
            tags.push("error".into());
        }
        Severity::Warning => tags.push("warning".into()),
        Severity::Notice | Severity::Info => tags.push("info".into()),
        Severity::Debug => tags.push("debug".into()),
    }

    let hay = format!("{identifier} {unit} {message}").to_ascii_lowercase();
    for (needle, tag) in SERVICE_HINTS {
        if hay.contains(needle) {
            tags.push((*tag).into());
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_line_string_message() {
        let line = r#"{"PRIORITY":"3","MESSAGE":"test fail","SYSLOG_IDENTIFIER":"hyprland","__REALTIME_TIMESTAMP":"1700000000000000","_SYSTEMD_USER_UNIT":"hyprland.service"}"#;
        let e = parse_line(line, 0).expect("parse");
        assert_eq!(e.severity, Severity::Error);
        assert_eq!(e.message, "test fail");
        assert!(e.tags.contains(&"error".into()));
        assert!(e.tags.contains(&"hyprland".into()));
    }

    #[test]
    fn parse_json_line_byte_message() {
        let line = r#"{"PRIORITY":"6","MESSAGE":[72,105],"SYSLOG_IDENTIFIER":"app"}"#;
        let e = parse_line(line, 0).expect("parse");
        assert_eq!(e.message, "Hi");
    }

    #[test]
    fn matches_query_and_tag() {
        let entry = LogEntry {
            id: 0,
            timestamp_display: String::new(),
            severity: Severity::Info,
            message: "PipeWire started".into(),
            identifier: "pipewire".into(),
            unit: "pipewire.service".into(),
            tags: vec!["info".into(), "pipewire".into()],
            raw_priority: 6,
        };
        assert!(matches(
            &entry,
            "pipewire",
            SeverityFilter::All,
            ""
        ));
        assert!(matches(&entry, "", SeverityFilter::All, "pipewire"));
        assert!(!matches(&entry, "", SeverityFilter::Errors, ""));
    }
}
