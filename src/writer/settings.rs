//! Managed `hl.config({...})` override block — single block that holds the
//! MVP-filtered merged settings table.

use std::path::Path;

use serde_json::Value;

use crate::settings_config;
use crate::spec;
use crate::writer::error::{ok_result, write_config_atomic, WriteError, WriteMode, WriteResult};
use crate::writer::section::{replace_managed_section_contents, MANAGED_CONFIG_BEGIN, MANAGED_CONFIG_END};

/// Rewrite the single managed-config override block with MVP-filtered merged settings.
pub fn save_config_override(
    config_path: &Path,
    merged: &Value,
) -> Result<WriteResult, WriteError> {
    if !config_path.is_file() {
        return Err(WriteError::Missing(config_path.display().to_string()));
    }
    let filtered = settings_config::filter_mvp(merged);
    let call = spec::format_config_call(&filtered);
    let source = std::fs::read_to_string(config_path)?;
    let updated = replace_managed_section_contents(
        &source,
        MANAGED_CONFIG_BEGIN,
        MANAGED_CONFIG_END,
        &call,
    )?;
    write_config_atomic(config_path, &updated)?;
    Ok(ok_result(
        config_path.display().to_string(),
        WriteMode::Appended,
    ))
}
