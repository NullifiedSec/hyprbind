//! RGB Studio — frame-based per-key animations and JSON presets.

use crate::experimental::via::{self, hsv_to_rgb, PaintBackend, ViaError, ViaResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_DELAY_MS: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };

    pub fn from_hsv(h: u8, s: u8, v: u8) -> Self {
        let (r, g, b) = hsv_to_rgb(h, s, v);
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellColor {
    pub row: u8,
    pub col: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CellColor {
    pub fn new(row: u8, col: u8, rgb: Rgb) -> Self {
        Self {
            row,
            col,
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }

    pub fn rgb(&self) -> Rgb {
        Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimFrame {
    #[serde(default = "default_delay")]
    pub delay_ms: u32,
    #[serde(default)]
    pub cells: Vec<CellColor>,
}

fn default_delay() -> u32 {
    DEFAULT_DELAY_MS
}

impl Default for AnimFrame {
    fn default() -> Self {
        Self {
            delay_ms: DEFAULT_DELAY_MS,
            cells: Vec::new(),
        }
    }
}

impl AnimFrame {
    pub fn cell_map(&self) -> HashMap<(u8, u8), Rgb> {
        self.cells
            .iter()
            .map(|c| ((c.row, c.col), c.rgb()))
            .collect()
    }

    pub fn set_cell(&mut self, row: u8, col: u8, rgb: Rgb) {
        if let Some(existing) = self.cells.iter_mut().find(|c| c.row == row && c.col == col) {
            existing.r = rgb.r;
            existing.g = rgb.g;
            existing.b = rgb.b;
        } else {
            self.cells.push(CellColor::new(row, col, rgb));
        }
    }

    pub fn clear_cells(&mut self) {
        self.cells.clear();
    }

    pub fn apply_map(&mut self, map: HashMap<(u8, u8), Rgb>) {
        self.cells = map
            .into_iter()
            .map(|((row, col), rgb)| CellColor::new(row, col, rgb))
            .collect();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    #[serde(default = "default_looped")]
    pub looped: bool,
    #[serde(default)]
    pub frames: Vec<AnimFrame>,
}

fn default_looped() -> bool {
    true
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            looped: true,
            frames: vec![AnimFrame::default()],
        }
    }
}

impl Animation {
    pub fn ensure_frame(&mut self) {
        if self.frames.is_empty() {
            self.frames.push(AnimFrame::default());
        }
    }

    pub fn frame(&self, idx: usize) -> Option<&AnimFrame> {
        self.frames.get(idx)
    }

    pub fn frame_mut(&mut self, idx: usize) -> Option<&mut AnimFrame> {
        self.frames.get_mut(idx)
    }
}

// ── Patterns ──────────────────────────────────────────────────────────────

/// Keys as `(row, col)` matrix cells.
pub type KeyList = [(u8, u8)];

pub fn pattern_fill(keys: &KeyList, color: Rgb) -> HashMap<(u8, u8), Rgb> {
    keys.iter().map(|&(r, c)| ((r, c), color)).collect()
}

pub fn pattern_clear(_keys: &KeyList) -> HashMap<(u8, u8), Rgb> {
    HashMap::new()
}

pub fn pattern_checkerboard(keys: &KeyList, a: Rgb, b: Rgb) -> HashMap<(u8, u8), Rgb> {
    keys.iter()
        .map(|&(row, col)| {
            let color = if (row + col) % 2 == 0 { a } else { b };
            ((row, col), color)
        })
        .collect()
}

pub fn pattern_gradient_h(keys: &KeyList, sat: u8, val: u8) -> HashMap<(u8, u8), Rgb> {
    let max_col = keys.iter().map(|(_, c)| *c).max().unwrap_or(0).max(1);
    keys.iter()
        .map(|&(row, col)| {
            let hue = ((col as u32 * 255) / max_col as u32) as u8;
            ((row, col), Rgb::from_hsv(hue, sat, val))
        })
        .collect()
}

pub fn pattern_rainbow_rows(keys: &KeyList, sat: u8, val: u8) -> HashMap<(u8, u8), Rgb> {
    let max_row = keys.iter().map(|(r, _)| *r).max().unwrap_or(0).max(1);
    keys.iter()
        .map(|&(row, col)| {
            let hue = ((row as u32 * 255) / max_row as u32) as u8;
            ((row, col), Rgb::from_hsv(hue, sat, val))
        })
        .collect()
}

// ── Presets I/O ───────────────────────────────────────────────────────────

pub fn presets_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hyprbinds")
        .join("rgb-presets")
}

pub fn ensure_presets_dir() -> ViaResult<PathBuf> {
    let dir = presets_dir();
    fs::create_dir_all(&dir).map_err(|e| ViaError::Message(format!("create {dir:?}: {e}")))?;
    Ok(dir)
}

fn sanitize_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() {
        "untitled".into()
    } else {
        s
    }
}

pub fn preset_path(name: &str) -> PathBuf {
    presets_dir().join(format!("{}.json", sanitize_name(name)))
}

pub fn list_presets() -> ViaResult<Vec<String>> {
    let dir = ensure_presets_dir()?;
    let mut names = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn save_preset(anim: &Animation) -> ViaResult<PathBuf> {
    let dir = ensure_presets_dir()?;
    let path = dir.join(format!("{}.json", sanitize_name(&anim.name)));
    let json = serde_json::to_string_pretty(anim)
        .map_err(|e| ViaError::Message(format!("serialize preset: {e}")))?;
    fs::write(&path, json).map_err(|e| ViaError::Message(format!("write {path:?}: {e}")))?;
    Ok(path)
}

pub fn load_preset(name: &str) -> ViaResult<Animation> {
    let path = preset_path(name);
    load_preset_path(&path)
}

pub fn load_preset_path(path: &Path) -> ViaResult<Animation> {
    let data = fs::read_to_string(path)
        .map_err(|e| ViaError::Message(format!("read {path:?}: {e}")))?;
    let mut anim: Animation = serde_json::from_str(&data)
        .map_err(|e| ViaError::Message(format!("parse preset: {e}")))?;
    anim.ensure_frame();
    if anim.name.trim().is_empty() {
        anim.name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
    }
    Ok(anim)
}

pub fn delete_preset(name: &str) -> ViaResult<()> {
    let path = preset_path(name);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| ViaError::Message(format!("delete {path:?}: {e}")))?;
    }
    Ok(())
}

/// Resolve frame cells to LED RGB tuples and push to the keyboard.
pub fn push_frame(
    vendor_id: u16,
    product_id: u16,
    backend: PaintBackend,
    frame: &AnimFrame,
    layout_order: &[(u8, u8)],
) -> ViaResult<()> {
    let mut leds: Vec<(u8, u8, u8, u8)> = Vec::new();
    for cell in &frame.cells {
        if let Some(index) =
            via::led_index_for_key(vendor_id, product_id, cell.row, cell.col, Some(layout_order))
        {
            leds.push((index, cell.r, cell.g, cell.b));
        }
    }
    via::set_leds_rgb(vendor_id, product_id, backend, &leds)
}
