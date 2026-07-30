//! Persist last-good config snapshots and restore them.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("source file missing: {0}")]
    Missing(String),
    #[error("no backup found for {0}")]
    NoBackup(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Sidecar next to the config: `hyprland.lua.hyprbinds.bak`
pub fn sidecar_backup(path: &Path) -> PathBuf {
    let mut backup = path.as_os_str().to_owned();
    backup.push(".hyprbinds.bak");
    PathBuf::from(backup)
}

/// Directory for timestamped snapshots: `~/.config/hyprbinds/backups/`
pub fn backups_dir() -> Option<PathBuf> {
    let mut dir = dirs::config_dir()?;
    dir.push("hyprbinds");
    dir.push("backups");
    Some(dir)
}

/// Called before overwriting `path`. Saves sidecar bak + a timestamped snapshot.
pub fn snapshot_before_write(path: &Path) -> Result<(), BackupError> {
    if !path.is_file() {
        return Ok(());
    }
    let bak = sidecar_backup(path);
    fs::copy(path, &bak)?;

    // Timestamped history is best-effort (may fail in sandboxes / missing home).
    if let Some(dir) = backups_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            let stem = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("config.lua");
            let ts = timestamp_slug();
            let dest = dir.join(format!("{stem}.{ts}.lua"));
            let _ = fs::copy(path, &dest);
            let _ = prune_old_backups(&dir, stem, 12);
        }
    }
    Ok(())
}

/// Restore the most recent good snapshot over `path`.
/// Prefers the newest timestamped backup; falls back to the sidecar `.bak`.
pub fn restore_last_good(path: &Path) -> Result<PathBuf, BackupError> {
    if !path.is_file() && !sidecar_backup(path).is_file() {
        // allow restore even if current missing, if bak exists
    }
    let source = newest_backup(path).ok_or_else(|| {
        BackupError::NoBackup(path.display().to_string())
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Keep a safety copy of whatever we're about to overwrite.
    if path.is_file() {
        let mut pre = path.as_os_str().to_owned();
        pre.push(".hyprbinds.prerestore");
        let _ = fs::copy(path, PathBuf::from(pre));
    }
    fs::copy(&source, path)?;
    Ok(source)
}

pub fn has_backup(path: &Path) -> bool {
    newest_backup(path).is_some()
}

pub fn newest_backup(path: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<(SystemTime, PathBuf)> = Vec::new();

    let bak = sidecar_backup(path);
    if bak.is_file() {
        if let Ok(meta) = bak.metadata() {
            if let Ok(mtime) = meta.modified() {
                candidates.push((mtime, bak));
            }
        }
    }

    if let Some(dir) = backups_dir() {
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("config.lua");
        let prefix = format!("{stem}.");
        if let Ok(entries) = fs::read_dir(dir) {
            for ent in entries.flatten() {
                let p = ent.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name.starts_with(&prefix) && name.ends_with(".lua") {
                    if let Ok(meta) = ent.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            candidates.push((mtime, p));
                        }
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, p)| p)
}

pub fn backup_age_label(path: &Path) -> Option<String> {
    let bak = newest_backup(path)?;
    let meta = bak.metadata().ok()?;
    let mtime = meta.modified().ok()?;
    let age = SystemTime::now().duration_since(mtime).ok()?;
    let secs = age.as_secs();
    let label = if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    };
    Some(format!("{} ({label})", bak.display()))
}

fn timestamp_slug() -> String {
    std::process::Command::new("date")
        .args(["+%Y%m%d-%H%M%S"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| format!("{}", unix_secs()))
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn prune_old_backups(dir: &Path, stem: &str, keep: usize) -> Result<(), BackupError> {
    let prefix = format!("{stem}.");
    let mut files: Vec<(SystemTime, PathBuf)> = Vec::new();
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let p = ent.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with(&prefix) && name.ends_with(".lua") {
            if let Ok(meta) = ent.metadata() {
                if let Ok(mtime) = meta.modified() {
                    files.push((mtime, p));
                }
            }
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, p) in files.into_iter().skip(keep) {
        let _ = fs::remove_file(p);
    }
    Ok(())
}
