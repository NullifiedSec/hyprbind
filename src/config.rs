use crate::bind::BindCollection;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

const COLLECT_SCRIPT: &str = include_str!("lua/collect_binds.lua");

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine Hyprland config path")]
    MissingConfigPath,
    #[error("config not found: {0}")]
    NotFound(PathBuf),
    #[error("failed to create temporary collector script: {0}")]
    TempScript(std::io::Error),
    #[error("failed to run lua collector: {0}")]
    Spawn(std::io::Error),
    #[error("lua collector exited with status {status}: {stderr}")]
    CollectorFailed { status: i32, stderr: String },
    #[error("failed to parse collector output: {0}")]
    Parse(#[from] serde_json::Error),
}

pub fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("hypr").join("hyprland.lua"))
}

pub fn load_binds(config_path: Option<PathBuf>) -> Result<BindCollection, ConfigError> {
    let path = config_path
        .or_else(default_config_path)
        .ok_or(ConfigError::MissingConfigPath)?;

    if !path.is_file() {
        return Err(ConfigError::NotFound(path));
    }

    let mut script = tempfile("hyprbinds-collect-", ".lua")?;
    script
        .file
        .write_all(COLLECT_SCRIPT.as_bytes())
        .map_err(ConfigError::TempScript)?;
    script.file.flush().map_err(ConfigError::TempScript)?;

    let output = Command::new("lua")
        .arg(&script.path)
        .arg(&path)
        .output()
        .map_err(ConfigError::Spawn)?;

    // Keep the temp file until the command finishes.
    drop(script);

    if !output.status.success() {
        return Err(ConfigError::CollectorFailed {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let collection: BindCollection = serde_json::from_slice(&output.stdout)?;
    Ok(collection)
}

struct TempScript {
    path: PathBuf,
    file: std::fs::File,
}

impl Drop for TempScript {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn tempfile(prefix: &str, suffix: &str) -> Result<TempScript, ConfigError> {
    let name = format!(
        "{}{}{}",
        prefix,
        std::process::id(),
        suffix
    );
    let path = std::env::temp_dir().join(name);
    let file = std::fs::File::create(&path).map_err(ConfigError::TempScript)?;
    Ok(TempScript { path, file })
}
