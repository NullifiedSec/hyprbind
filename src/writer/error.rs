use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("source file missing: {0}")]
    Missing(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not find call near line {line}")]
    CallNotFound { line: u32 },
    #[error("could not find variable declaration for {name}")]
    VariableNotFound { name: String },
    #[error("invalid edit: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct WriteResult {
    pub path: String,
    pub mode: WriteMode,
    pub updated_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    InPlace,
    Materialized,
    Appended,
}

pub(crate) fn ok_result(path: impl Into<String>, mode: WriteMode) -> WriteResult {
    let path = path.into();
    WriteResult {
        updated_files: vec![path.clone()],
        path,
        mode,
    }
}

pub(crate) fn ok_result_many(
    primary: impl Into<String>,
    mode: WriteMode,
    files: Vec<String>,
) -> WriteResult {
    WriteResult {
        path: primary.into(),
        mode,
        updated_files: files,
    }
}

/// Write config safely: snapshot last-good, then atomic replace via temp file.
pub(crate) fn write_config_atomic(path: &Path, contents: &str) -> Result<(), WriteError> {
    crate::backup::snapshot_before_write(path)
        .map_err(|e| WriteError::Invalid(format!("backup failed: {e}")))?;

    let tmp = path.with_extension("lua.hyprbinds.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
