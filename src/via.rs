//! VIA/QMK hardware keymap support.
//!
//! Talks to VIA-enabled keyboards over USB HID (usage page `0xFF60`) using the
//! same dynamic-keymap commands as the official VIA configurator — no firmware
//! flashing. Keyboard geometry comes from sideloaded VIA definition JSON files
//! (see https://caniusevia.com/docs/specification/).
//!
//! Also supports Vial firmware: onboard definition fetch, tap dance, combos,
//! and key overrides. Public APIs below are ready for a UI agent; stock VIA
//! path (JSON definitions, keymap, lighting) is unchanged.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use via_protocol::device::{
    check_hid_permissions, discover_keyboards, HidAccessStatus, KeyboardDevice, KeyboardInfo,
};
use via_protocol::layout::{parse_vial_definition, KeyboardLayout};
use via_protocol::{
    ComboEntry as ProtoComboEntry, DynamicEntryCounts, KeyOverrideEntry as ProtoKeyOverrideEntry,
    Keycode, LightingProtocol, LightingValues, TapDanceEntry as ProtoTapDanceEntry, ViaCommand,
    ViaCommandId, ViaProtocol,
};

/// QMK `QK_KB_0` — first keyboard-level custom keycode (VIA `customKeycodes[0]`).
pub const QK_KB_0: u16 = 0x7E00;

#[derive(Debug, Error)]
pub enum ViaError {
    #[error("{0}")]
    Message(String),
    #[error("HID: {0}")]
    Hid(String),
    #[error("definition: {0}")]
    Definition(String),
}

pub type ViaResult<T> = Result<T, ViaError>;

#[derive(Debug, Clone)]
pub struct CustomKeycode {
    pub name: String,
    pub title: String,
    pub short_name: String,
}

#[derive(Debug, Clone)]
pub struct ViaDefinition {
    pub path: PathBuf,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub rows: u8,
    pub cols: u8,
    pub custom_keycodes: Vec<CustomKeycode>,
    pub layout: KeyboardLayout,
    /// Effect names from the definition menus (empty = probe-only / numbered).
    pub lighting: LightingMenuInfo,
}

#[derive(Debug, Clone, Default)]
pub struct LightingMenuInfo {
    /// Definition declares a lighting menu (or built-in qmk_* lighting menu).
    pub declared: bool,
    pub effects: Vec<(String, u16)>,
    pub has_brightness: bool,
    pub has_speed: bool,
    pub has_color: bool,
}

#[derive(Debug, Clone)]
pub struct LightingSnapshot {
    pub label: String,
    pub brightness: u8,
    pub effect: u16,
    pub speed: u8,
    pub hue: u8,
    pub saturation: u8,
}

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: String,
    pub product: String,
    #[allow(dead_code)]
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct KeymapSnapshot {
    pub protocol: u16,
    pub layers: u8,
    /// `[layer][row][col]`
    pub map: Vec<Vec<Vec<u16>>>,
}

/// Vial keyboard identity from `vial_get_keyboard_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VialInfo {
    pub protocol_version: u32,
    pub uid: [u8; 8],
}

/// Dynamic entry slot counts (mirrors firmware `DynamicEntryCounts`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DynamicCounts {
    pub tap_dance: u8,
    pub combo: u8,
    pub key_override: u8,
    pub alt_repeat: u8,
}

impl From<DynamicEntryCounts> for DynamicCounts {
    fn from(c: DynamicEntryCounts) -> Self {
        Self {
            tap_dance: c.tap_dance,
            combo: c.combo,
            key_override: c.key_override,
            alt_repeat: c.alt_repeat,
        }
    }
}

/// Tap dance slot (10-byte Vial EEPROM layout).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TapDance {
    pub on_tap: u16,
    pub on_hold: u16,
    pub on_double_tap: u16,
    pub on_tap_hold: u16,
    pub tapping_term: u16,
}

impl From<ProtoTapDanceEntry> for TapDance {
    fn from(e: ProtoTapDanceEntry) -> Self {
        Self {
            on_tap: e.on_tap,
            on_hold: e.on_hold,
            on_double_tap: e.on_double_tap,
            on_tap_hold: e.on_tap_hold,
            tapping_term: e.tapping_term,
        }
    }
}

impl From<&TapDance> for ProtoTapDanceEntry {
    fn from(e: &TapDance) -> Self {
        Self {
            on_tap: e.on_tap,
            on_hold: e.on_hold,
            on_double_tap: e.on_double_tap,
            on_tap_hold: e.on_tap_hold,
            tapping_term: e.tapping_term,
        }
    }
}

/// Combo slot (up to 4 input keys → one output).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combo {
    pub input: [u16; 4],
    pub output: u16,
}

impl From<ProtoComboEntry> for Combo {
    fn from(e: ProtoComboEntry) -> Self {
        Self {
            input: e.input,
            output: e.output,
        }
    }
}

impl From<&Combo> for ProtoComboEntry {
    fn from(e: &Combo) -> Self {
        Self {
            input: e.input,
            output: e.output,
        }
    }
}

/// Key override slot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyOverride {
    pub trigger: u16,
    pub replacement: u16,
    pub layers: u16,
    pub trigger_mods: u8,
    pub negative_mod_mask: u8,
    pub suppressed_mods: u8,
    /// Bit 7 = enabled.
    pub options: u8,
}

impl From<ProtoKeyOverrideEntry> for KeyOverride {
    fn from(e: ProtoKeyOverrideEntry) -> Self {
        Self {
            trigger: e.trigger,
            replacement: e.replacement,
            layers: e.layers,
            trigger_mods: e.trigger_mods,
            negative_mod_mask: e.negative_mod_mask,
            suppressed_mods: e.suppressed_mods,
            options: e.options,
        }
    }
}

impl From<&KeyOverride> for ProtoKeyOverrideEntry {
    fn from(e: &KeyOverride) -> Self {
        Self {
            trigger: e.trigger,
            replacement: e.replacement,
            layers: e.layers,
            trigger_mods: e.trigger_mods,
            negative_mod_mask: e.negative_mod_mask,
            suppressed_mods: e.suppressed_mods,
            options: e.options,
        }
    }
}

/// Probed feature set for a connected board (Vial vs stock VIA).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoardCapabilities {
    pub is_vial: bool,
    pub tap_dance: u8,
    pub combo: u8,
    pub key_override: u8,
    pub lighting_label: Option<String>,
    /// Backend usable for per-key RGB paint (OpenRGB offset or VialRGB DirectFastSet).
    pub paint_backend: PaintBackend,
}

/// How per-key LED writes are sent over RAW HID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PaintBackend {
    #[default]
    None,
    /// Remaster OpenRGB-merged protocol (`cmd + 0x20`), RGB bytes.
    OpenRgbOffset,
    /// Stock VialRGB DirectFastSet (`CustomSetValue` + `0x42`), HSV bytes.
    VialRgbDirect,
}

impl PaintBackend {
    pub fn is_available(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "unavailable",
            Self::OpenRgbOffset => "OpenRGB offset",
            Self::VialRgbDirect => "VialRGB DirectFastSet",
        }
    }
}

/// Royal Kludge R75 remaster USB IDs (`VID:PID 342d:e484`).
pub const R75_VID: u16 = 0x342d;
pub const R75_PID: u16 = 0xe484;

const OPENRGB_MERGED_OFFSET: u8 = 0x20;
const OPENRGB_GET_PROTOCOL_VERSION: u8 = 1;
const OPENRGB_DIRECT_MODE_SET_SINGLE: u8 = 8;
const OPENRGB_DIRECT_MODE_SET_LEDS: u8 = 9;
const OPENRGB_PROTOCOL_VERSION: u8 = 0x0e;
/// Max LEDs per OpenRGB `DIRECT_MODE_SET_LEDS` packet (3 bytes RGB each after header).
const OPENRGB_MAX_LEDS_PER_PACKET: usize = 9;
/// VialRGB sub-command shared by GetSupported (GET) / DirectFastSet (SET).
const VIALRGB_DIRECT_FASTSET: u8 = 0x42;
const VIALRGB_EFFECT_DIRECT: u16 = 1;
const VIALRGB_EFFECT_SOLID_COLOR: u16 = 2;
const QMK_EFFECT_SOLID_COLOR: u16 = 1;

/// `g_led_config.matrix_co` for R75 (`overlay/keyboards/rk/r75/keyboard.json` rgb_matrix.layout).
/// Index is `[row][col]` → LED index, or `None` when that matrix cell has no LED.
pub const R75_MATRIX_TO_LED: [[Option<u8>; 15]; 6] = [
    [
        Some(21),
        Some(20),
        Some(19),
        Some(18),
        Some(17),
        Some(16),
        Some(15),
        Some(14),
        Some(13),
        Some(12),
        Some(11),
        Some(10),
        Some(9),
        Some(8),
        None,
    ],
    [
        Some(22),
        Some(23),
        Some(24),
        Some(25),
        Some(26),
        Some(27),
        Some(28),
        Some(29),
        Some(30),
        Some(31),
        Some(32),
        Some(33),
        Some(34),
        Some(35),
        Some(7),
    ],
    [
        Some(49),
        Some(48),
        Some(47),
        Some(46),
        Some(45),
        Some(44),
        Some(43),
        Some(42),
        Some(41),
        Some(40),
        Some(39),
        Some(38),
        Some(37),
        Some(36),
        Some(6),
    ],
    [
        Some(50),
        Some(51),
        Some(52),
        Some(53),
        Some(54),
        Some(55),
        Some(56),
        Some(57),
        Some(58),
        Some(59),
        Some(60),
        Some(61),
        None,
        Some(62),
        Some(5),
    ],
    [
        Some(75),
        Some(74),
        Some(73),
        Some(72),
        Some(71),
        Some(70),
        Some(69),
        Some(68),
        Some(67),
        Some(66),
        Some(65),
        Some(64),
        None,
        Some(63),
        None,
    ],
    [
        Some(76),
        Some(77),
        Some(78),
        None,
        None,
        Some(79),
        None,
        None,
        None,
        Some(0),
        Some(1),
        None,
        Some(2),
        Some(3),
        Some(4),
    ],
];

#[derive(Deserialize)]
struct DefinitionMeta {
    name: Option<String>,
    #[serde(default, rename = "vendorId", deserialize_with = "deserialize_optional_hex_id")]
    vendor_id: Option<u16>,
    #[serde(default, rename = "productId", deserialize_with = "deserialize_optional_hex_id")]
    product_id: Option<u16>,
    matrix: MatrixMeta,
    #[serde(default, rename = "customKeycodes")]
    custom_keycodes: Vec<CustomKeycodeMeta>,
    #[serde(default)]
    menus: Value,
}

#[derive(Deserialize)]
struct MatrixMeta {
    rows: u8,
    cols: u8,
}

#[derive(Deserialize)]
struct CustomKeycodeMeta {
    name: Option<String>,
    title: Option<String>,
    #[serde(rename = "shortName")]
    short_name: Option<String>,
}

pub fn definitions_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hyprbinds")
        .join("via-definitions")
}

/// Ensure the definitions directory exists and seed bundled JSON if empty.
pub fn ensure_definitions() -> ViaResult<PathBuf> {
    let dir = definitions_dir();
    fs::create_dir_all(&dir).map_err(|e| ViaError::Message(format!("create {dir:?}: {e}")))?;

    let has_json = fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        })
        .unwrap_or(false);

    if !has_json {
        for candidate in bundled_definition_sources() {
            if candidate.is_dir() {
                if let Ok(rd) = fs::read_dir(&candidate) {
                    for entry in rd.filter_map(|e| e.ok()) {
                        let src = entry.path();
                        if src.extension().and_then(|x| x.to_str()) == Some("json") {
                            let dest = dir.join(src.file_name().unwrap_or_default());
                            let _ = fs::copy(&src, &dest);
                        }
                    }
                }
            } else if candidate.is_file() {
                let dest = dir.join(candidate.file_name().unwrap_or_default());
                let _ = fs::copy(&candidate, &dest);
            }
        }
    }

    Ok(dir)
}

fn bundled_definition_sources() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("via-definitions"));
        out.push(cwd.join("R75 Wired Windows QMK.json"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            out.push(parent.join("via-definitions"));
            out.push(parent.join("../via-definitions"));
        }
    }
    out
}

pub fn list_definitions() -> ViaResult<Vec<ViaDefinition>> {
    let dir = ensure_definitions()?;
    let mut defs = Vec::new();
    let rd = fs::read_dir(&dir).map_err(|e| ViaError::Message(format!("read {dir:?}: {e}")))?;
    for entry in rd.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        match load_definition(&path) {
            Ok(def) => defs.push(def),
            Err(e) => eprintln!("hyprbinds: skip VIA definition {}: {e}", path.display()),
        }
    }
    defs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(defs)
}

pub fn load_definition(path: &Path) -> ViaResult<ViaDefinition> {
    let text = fs::read_to_string(path)
        .map_err(|e| ViaError::Definition(format!("read {}: {e}", path.display())))?;
    parse_definition_json(&text, path.to_path_buf())
}

pub fn import_definition(src: &Path) -> ViaResult<ViaDefinition> {
    let dir = ensure_definitions()?;
    let name = src
        .file_name()
        .ok_or_else(|| ViaError::Message("invalid definition path".into()))?;
    let dest = dir.join(name);
    fs::copy(src, &dest).map_err(|e| ViaError::Message(format!("copy to {dest:?}: {e}")))?;
    load_definition(&dest)
}

fn parse_definition_json(text: &str, path: PathBuf) -> ViaResult<ViaDefinition> {
    parse_definition_json_with_ids(text, path, None)
}

/// Parse definition JSON; when `ids` is `Some`, those VID/PID override the file.
fn parse_definition_json_with_ids(
    text: &str,
    path: PathBuf,
    ids: Option<(u16, u16)>,
) -> ViaResult<ViaDefinition> {
    let meta: DefinitionMeta = serde_json::from_str(text)
        .map_err(|e| ViaError::Definition(format!("{}: {e}", path.display())))?;
    let layout = parse_vial_definition(text)
        .map_err(|e| ViaError::Definition(format!("{}: {e}", path.display())))?;

    let (vendor_id, product_id) = match ids {
        Some(pair) => pair,
        None => {
            let vid = meta.vendor_id.ok_or_else(|| {
                ViaError::Definition(format!("{}: missing vendorId", path.display()))
            })?;
            let pid = meta.product_id.ok_or_else(|| {
                ViaError::Definition(format!("{}: missing productId", path.display()))
            })?;
            (vid, pid)
        }
    };

    // Prefer metadata name; fill vid/pid on the layout for matching.
    let name = meta
        .name
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| layout.name.clone());

    let custom_keycodes = meta
        .custom_keycodes
        .into_iter()
        .enumerate()
        .map(|(i, c)| CustomKeycode {
            name: c.name.unwrap_or_else(|| format!("CUSTOM_{i}")),
            title: c.title.unwrap_or_default(),
            short_name: c
                .short_name
                .unwrap_or_else(|| format!("C{i}")),
        })
        .collect();

    let mut layout = layout;
    layout.name = name.clone();
    layout.vid_pid = vec![(vendor_id, product_id)];
    layout.rows = meta.matrix.rows;
    layout.cols = meta.matrix.cols;

    let lighting = parse_lighting_menu(&meta.menus);

    Ok(ViaDefinition {
        path,
        name,
        vendor_id,
        product_id,
        rows: meta.matrix.rows,
        cols: meta.matrix.cols,
        custom_keycodes,
        layout,
        lighting,
    })
}

/// Standard QMK RGB Matrix effect names (matches VIA built-in `qmk_rgb_matrix` menu).
pub fn default_rgb_matrix_effects() -> Vec<(String, u16)> {
    [
        "All Off",
        "Solid Color",
        "Alphas Mods",
        "Gradient Up Down",
        "Gradient Left Right",
        "Breathing",
        "Band Sat",
        "Band Val",
        "Band Pinwheel Sat",
        "Band Pinwheel Val",
        "Band Spiral Sat",
        "Band Spiral Val",
        "Cycle All",
        "Cycle Left Right",
        "Cycle Up Down",
        "Rainbow Moving Chevron",
        "Cycle Out In",
        "Cycle Out In Dual",
        "Cycle Pinwheel",
        "Cycle Spiral",
        "Dual Beacon",
        "Rainbow Beacon",
        "Rainbow Pinwheels",
        "Raindrops",
        "Jellybean Raindrops",
        "Hue Breathing",
        "Hue Pendulum",
        "Hue Wave",
        "Pixel Rain",
        "Pixel Flow",
        "Pixel Fractal",
        "Typing Heatmap",
        "Digital Rain",
        "Solid Reactive Simple",
        "Solid Reactive",
        "Solid Reactive Wide",
        "Solid Reactive Multiwide",
        "Solid Reactive Cross",
        "Solid Reactive Multicross",
        "Solid Reactive Nexus",
        "Solid Reactive Multinexus",
        "Splash",
        "Multisplash",
        "Solid Splash",
        "Solid Multisplash",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, name)| (name.to_string(), i as u16))
    .collect()
}

fn parse_lighting_menu(menus: &Value) -> LightingMenuInfo {
    let mut info = LightingMenuInfo::default();
    match menus {
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(s) => match s.as_str() {
                        "qmk_rgb_matrix" | "qmk_rgblight" | "qmk_backlight"
                        | "qmk_backlight_rgblight" => {
                            info.declared = true;
                            info.has_brightness = true;
                            info.has_speed = s.contains("rgb");
                            info.has_color = s.contains("rgb");
                            if info.effects.is_empty() && s.contains("rgb_matrix") {
                                info.effects = default_rgb_matrix_effects();
                            }
                        }
                        _ => {}
                    },
                    Value::Object(_) => walk_menu_controls(item, &mut info),
                    _ => {}
                }
            }
        }
        Value::Object(_) => walk_menu_controls(menus, &mut info),
        _ => {}
    }
    if info.declared && info.effects.is_empty() {
        info.effects = default_rgb_matrix_effects();
    }
    info
}

fn walk_menu_controls(node: &Value, info: &mut LightingMenuInfo) {
    let Some(obj) = node.as_object() else {
        return;
    };
    if let Some(t) = obj.get("type").and_then(|v| v.as_str()) {
        let content_id = obj
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let is_light = content_id.contains("rgb")
            || content_id.contains("backlight")
            || content_id.contains("lighting");
        if is_light {
            info.declared = true;
            match t {
                "range" if content_id.contains("brightness") => info.has_brightness = true,
                "range" if content_id.contains("speed") => info.has_speed = true,
                "dropdown" if content_id.contains("effect") => {
                    if let Some(opts) = obj.get("options").and_then(|v| v.as_array()) {
                        let mut effects = Vec::new();
                        for opt in opts {
                            if let Some(arr) = opt.as_array() {
                                let label = arr
                                    .first()
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Effect")
                                    .to_string();
                                let id = arr
                                    .get(1)
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(effects.len() as u64)
                                    as u16;
                                effects.push((label, id));
                            }
                        }
                        if !effects.is_empty() {
                            info.effects = effects;
                        }
                    }
                }
                "color" => info.has_color = true,
                _ => {}
            }
        }
    }
    if let Some(content) = obj.get("content") {
        match content {
            Value::Array(items) => {
                for item in items {
                    if item.is_object() || item.is_array() {
                        walk_menu_controls(item, info);
                    }
                }
            }
            other => walk_menu_controls(other, info),
        }
    }
}

/// Effects list for the UI: definition menu first, else standard RGB matrix names.
pub fn effects_for_definition(def: &ViaDefinition) -> Vec<(String, u16)> {
    if !def.lighting.effects.is_empty() {
        def.lighting.effects.clone()
    } else {
        default_rgb_matrix_effects()
    }
}

fn lighting_proto_label(proto: &LightingProtocol) -> String {
    match proto {
        LightingProtocol::Via { channel } => format!("VIA {channel:?}"),
        LightingProtocol::VialLegacy => "Vial legacy".into(),
        LightingProtocol::VialRgb => "VialRGB".into(),
    }
}

pub fn read_lighting(vendor_id: u16, product_id: u16) -> ViaResult<Option<LightingSnapshot>> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let Some(lp) = proto.detect_lighting_protocol() else {
        return Ok(None);
    };
    let vals = proto
        .read_lighting_values(&lp)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(Some(LightingSnapshot {
        label: lighting_proto_label(&lp),
        brightness: vals.brightness,
        effect: vals.effect_id,
        speed: vals.speed,
        hue: vals.hue,
        saturation: vals.saturation,
    }))
}

pub fn write_lighting(vendor_id: u16, product_id: u16, snap: &LightingSnapshot) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let lp = proto
        .detect_lighting_protocol()
        .ok_or_else(|| ViaError::Message("keyboard has no VIA lighting channel".into()))?;
    let vals = LightingValues {
        effect_id: snap.effect,
        brightness: snap.brightness,
        speed: snap.speed,
        hue: snap.hue,
        saturation: snap.saturation,
    };
    proto
        .write_lighting_values(&lp, &vals)
        .map_err(|e| ViaError::Hid(e.to_string()))
}

pub fn save_lighting(vendor_id: u16, product_id: u16) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let lp = proto
        .detect_lighting_protocol()
        .ok_or_else(|| ViaError::Message("keyboard has no VIA lighting channel".into()))?;
    proto
        .save_lighting(&lp)
        .map_err(|e| ViaError::Hid(e.to_string()))
}

/// Probe OpenRGB-offset (0x20) then VialRGB for a usable per-key paint backend.
pub fn detect_paint_backend(vendor_id: u16, product_id: u16) -> ViaResult<PaintBackend> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    if probe_openrgb_offset(&device) {
        return Ok(PaintBackend::OpenRgbOffset);
    }
    let proto = ViaProtocol::new(&device);
    if matches!(
        proto.detect_lighting_protocol(),
        Some(LightingProtocol::VialRgb)
    ) {
        return Ok(PaintBackend::VialRgbDirect);
    }
    Ok(PaintBackend::None)
}

fn probe_openrgb_offset(device: &via_protocol::device::KeyboardDevice) -> bool {
    let mut report = [0u8; 33];
    report[1] = OPENRGB_MERGED_OFFSET + OPENRGB_GET_PROTOCOL_VERSION;
    match device.raw_hid_send(&report) {
        Ok(resp) => {
            resp[0] == OPENRGB_MERGED_OFFSET + OPENRGB_GET_PROTOCOL_VERSION
                && resp[1] == OPENRGB_PROTOCOL_VERSION
        }
        Err(_) => false,
    }
}

/// Map matrix (row, col) → RGB matrix LED index for known boards.
///
/// R75 remaster uses the firmware `rgb_matrix.layout` table. Other boards fall
/// back to layout-key order index when `layout_order` is provided (key index in
/// VIA definition layout order).
pub fn led_index_for_key(
    vendor_id: u16,
    product_id: u16,
    row: u8,
    col: u8,
    layout_order: Option<&[(u8, u8)]>,
) -> Option<u8> {
    if vendor_id == R75_VID && product_id == R75_PID {
        return R75_MATRIX_TO_LED
            .get(row as usize)
            .and_then(|cols| cols.get(col as usize))
            .copied()
            .flatten();
    }
    layout_order.and_then(|keys| {
        keys.iter()
            .position(|&(r, c)| r == row && c == col)
            .and_then(|i| u8::try_from(i).ok())
    })
}

/// Convert QMK/VIA 0–255 HSV to 8-bit RGB (spectrum wheel).
pub fn hsv_to_rgb(h: u8, s: u8, v: u8) -> (u8, u8, u8) {
    let h = h as f64 / 255.0 * 360.0;
    let s = s as f64 / 255.0;
    let v = v as f64 / 255.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

/// Switch firmware into a mode that accepts per-key paints.
pub fn enter_paint_mode(
    vendor_id: u16,
    product_id: u16,
    backend: PaintBackend,
    brightness: u8,
) -> ViaResult<()> {
    match backend {
        PaintBackend::None => Err(ViaError::Message(
            "this keyboard has no per-key paint backend".into(),
        )),
        // OpenRGB SET_SINGLE flips direct mode on the first paint.
        PaintBackend::OpenRgbOffset => Ok(()),
        PaintBackend::VialRgbDirect => {
            let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
            let (_info, device) = open_matching(&api, vendor_id, product_id)?;
            // Direct effect ignores HSV for effects, but values are still required.
            device
                .send_command(&ViaCommand::vialrgb_set_mode(
                    VIALRGB_EFFECT_DIRECT,
                    128,
                    0,
                    255,
                    brightness.max(1),
                ))
                .map_err(|e| ViaError::Hid(e.to_string()))?;
            Ok(())
        }
    }
}

/// Leave direct/paint mode by applying Solid Color through the normal lighting path.
pub fn exit_paint_mode(
    vendor_id: u16,
    product_id: u16,
    snap: &LightingSnapshot,
) -> ViaResult<LightingSnapshot> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let lp = proto
        .detect_lighting_protocol()
        .ok_or_else(|| ViaError::Message("keyboard has no VIA lighting channel".into()))?;
    let solid = match lp {
        LightingProtocol::VialRgb => VIALRGB_EFFECT_SOLID_COLOR,
        _ => QMK_EFFECT_SOLID_COLOR,
    };
    let out = LightingSnapshot {
        label: lighting_proto_label(&lp),
        brightness: snap.brightness.max(1),
        effect: solid,
        speed: snap.speed,
        hue: snap.hue,
        saturation: snap.saturation,
    };
    let vals = LightingValues {
        effect_id: out.effect,
        brightness: out.brightness,
        speed: out.speed,
        hue: out.hue,
        saturation: out.saturation,
    };
    proto
        .write_lighting_values(&lp, &vals)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(out)
}

/// Paint one LED. `index` is the RGB matrix LED index (not matrix row/col).
pub fn set_led_hsv(
    vendor_id: u16,
    product_id: u16,
    backend: PaintBackend,
    index: u8,
    h: u8,
    s: u8,
    v: u8,
) -> ViaResult<()> {
    match backend {
        PaintBackend::None => Err(ViaError::Message(
            "this keyboard has no per-key paint backend".into(),
        )),
        PaintBackend::OpenRgbOffset => {
            let (r, g, b) = hsv_to_rgb(h, s, v);
            set_led_openrgb_rgb(vendor_id, product_id, index, r, g, b)
        }
        PaintBackend::VialRgbDirect => {
            set_led_vialrgb_hsv(vendor_id, product_id, index, h, s, v)
        }
    }
}

/// Paint one LED from 8-bit RGB.
pub fn set_led_rgb(
    vendor_id: u16,
    product_id: u16,
    backend: PaintBackend,
    index: u8,
    r: u8,
    g: u8,
    b: u8,
) -> ViaResult<()> {
    match backend {
        PaintBackend::None => Err(ViaError::Message(
            "this keyboard has no per-key paint backend".into(),
        )),
        PaintBackend::OpenRgbOffset => set_led_openrgb_rgb(vendor_id, product_id, index, r, g, b),
        PaintBackend::VialRgbDirect => {
            let (h, s, v) = rgb_to_hsv(r, g, b);
            set_led_vialrgb_hsv(vendor_id, product_id, index, h, s, v)
        }
    }
}

/// Push many LEDs as `(index, r, g, b)`. OpenRGB batches ≤9 per packet; VialRGB
/// falls back to per-LED DirectFastSet.
pub fn set_leds_rgb(
    vendor_id: u16,
    product_id: u16,
    backend: PaintBackend,
    leds: &[(u8, u8, u8, u8)],
) -> ViaResult<()> {
    if leds.is_empty() {
        return Ok(());
    }
    match backend {
        PaintBackend::None => Err(ViaError::Message(
            "this keyboard has no per-key paint backend".into(),
        )),
        PaintBackend::OpenRgbOffset => set_leds_openrgb_rgb(vendor_id, product_id, leds),
        PaintBackend::VialRgbDirect => {
            for &(index, r, g, b) in leds {
                let (h, s, v) = rgb_to_hsv(r, g, b);
                set_led_vialrgb_hsv(vendor_id, product_id, index, h, s, v)?;
            }
            Ok(())
        }
    }
}

/// Convert 8-bit RGB to QMK/VIA 0–255 HSV.
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let v = max;
    let s = if max <= f64::EPSILON {
        0.0
    } else {
        delta / max
    };
    let h = if delta <= f64::EPSILON {
        0.0
    } else if (max - rf).abs() < f64::EPSILON {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < f64::EPSILON {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (
        ((h / 360.0) * 255.0).round().clamp(0.0, 255.0) as u8,
        (s * 255.0).round().clamp(0.0, 255.0) as u8,
        (v * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn set_led_openrgb_rgb(
    vendor_id: u16,
    product_id: u16,
    index: u8,
    r: u8,
    g: u8,
    b: u8,
) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let mut report = [0u8; 33];
    report[1] = OPENRGB_MERGED_OFFSET + OPENRGB_DIRECT_MODE_SET_SINGLE;
    report[2] = index;
    report[3] = r;
    report[4] = g;
    report[5] = b;
    device
        .raw_hid_send(&report)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(())
}

/// OpenRGB batch write: contiguous runs by LED index, ≤9 LEDs per packet.
fn set_leds_openrgb_rgb(
    vendor_id: u16,
    product_id: u16,
    leds: &[(u8, u8, u8, u8)],
) -> ViaResult<()> {
    let mut sorted = leds.to_vec();
    sorted.sort_by_key(|(index, _, _, _)| *index);

    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;

    let mut i = 0;
    while i < sorted.len() {
        let first = sorted[i].0;
        let mut run: Vec<(u8, u8, u8)> = Vec::new();
        while i < sorted.len() {
            let (idx, r, g, b) = sorted[i];
            let expect = first.saturating_add(run.len() as u8);
            if idx != expect {
                break;
            }
            run.push((r, g, b));
            i += 1;
        }
        let mut offset = 0;
        while offset < run.len() {
            let n = (run.len() - offset).min(OPENRGB_MAX_LEDS_PER_PACKET);
            let mut report = [0u8; 33];
            report[1] = OPENRGB_MERGED_OFFSET + OPENRGB_DIRECT_MODE_SET_LEDS;
            report[2] = first.saturating_add(offset as u8);
            report[3] = n as u8;
            for (k, (r, g, b)) in run[offset..offset + n].iter().enumerate() {
                let base = 4 + k * 3;
                report[base] = *r;
                report[base + 1] = *g;
                report[base + 2] = *b;
            }
            device
                .raw_hid_send(&report)
                .map_err(|e| ViaError::Hid(e.to_string()))?;
            offset += n;
        }
    }
    Ok(())
}

fn set_led_vialrgb_hsv(
    vendor_id: u16,
    product_id: u16,
    index: u8,
    h: u8,
    s: u8,
    v: u8,
) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    // CustomSetValue + DirectFastSet: [0x42, start_led u16 LE, num, HSV…]
    let cmd = ViaCommand::with_data(
        ViaCommandId::CustomSetValue,
        &[
            VIALRGB_DIRECT_FASTSET,
            index,
            0,
            1,
            h,
            s,
            v,
        ],
    );
    device
        .send_command(&cmd)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(())
}

/// QMK RGB matrix effects where color picker is usually hidden (off / rainbow / random).
pub fn effect_hides_color(effect: u16) -> bool {
    matches!(effect, 0 | 24 | 28 | 29 | 32)
}

fn parse_hex_id(s: &str) -> ViaResult<u16> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(s, 16).map_err(|_| ViaError::Definition(format!("bad id '{s}'")))
}

/// Accept `"0x342d"`, `"342d"`, or numeric JSON for optional VID/PID fields.
fn deserialize_optional_hex_id<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                parse_hex_id(&s)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
        }
        Some(Value::Number(n)) => {
            let v = n
                .as_u64()
                .ok_or_else(|| serde::de::Error::custom("id out of range"))?;
            if v > u16::MAX as u64 {
                return Err(serde::de::Error::custom("id out of range"));
            }
            Ok(Some(v as u16))
        }
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected hex string or number, got {other}"
        ))),
    }
}

/// Soft-detect Vial firmware. Returns `None` for stock VIA boards (or when the
/// Vial ID command fails / reports an invalid protocol version).
pub fn detect_vial(vendor_id: u16, product_id: u16) -> ViaResult<Option<VialInfo>> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    match proto.vial_get_keyboard_id() {
        Ok((version, uid)) if version > 0 => Ok(Some(VialInfo {
            protocol_version: version,
            uid,
        })),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Fetch the onboard Vial definition JSON, parse it into a `ViaDefinition`, and
/// optionally cache a copy under the definitions directory.
///
/// Works even when lighting menus are absent. VID/PID come from the open device.
pub fn fetch_vial_definition(vendor_id: u16, product_id: u16) -> ViaResult<ViaDefinition> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let json = proto
        .vial_get_definition()
        .map_err(|e| ViaError::Hid(e.to_string()))?;

    let dir = ensure_definitions()?;
    let cache_name = format!("vial-fetched-{vendor_id:04x}-{product_id:04x}.json");
    let cache_path = dir.join(&cache_name);
    if let Err(e) = fs::write(&cache_path, &json) {
        eprintln!(
            "hyprbinds: could not cache Vial definition {}: {e}",
            cache_path.display()
        );
    }

    let path = if cache_path.is_file() {
        cache_path
    } else {
        PathBuf::from(format!("vial://{vendor_id:04x}:{product_id:04x}"))
    };

    parse_definition_json_with_ids(&json, path, Some((vendor_id, product_id)))
}

pub fn read_dynamic_counts(vendor_id: u16, product_id: u16) -> ViaResult<DynamicCounts> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let counts = proto
        .get_dynamic_entry_counts()
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(counts.into())
}

pub fn read_tap_dances(vendor_id: u16, product_id: u16) -> ViaResult<Vec<TapDance>> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let counts = proto
        .get_dynamic_entry_counts()
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    let entries = proto
        .get_all_tap_dances(counts.tap_dance)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(entries.into_iter().map(TapDance::from).collect())
}

pub fn write_tap_dance(
    vendor_id: u16,
    product_id: u16,
    idx: u8,
    entry: &TapDance,
) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let proto_entry = ProtoTapDanceEntry::from(entry);
    proto
        .set_tap_dance(idx, &proto_entry)
        .map_err(|e| ViaError::Hid(e.to_string()))
}

pub fn read_combos(vendor_id: u16, product_id: u16) -> ViaResult<Vec<Combo>> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let counts = proto
        .get_dynamic_entry_counts()
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    let entries = proto
        .get_all_combos(counts.combo)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(entries.into_iter().map(Combo::from).collect())
}

pub fn write_combo(vendor_id: u16, product_id: u16, idx: u8, entry: &Combo) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let proto_entry = ProtoComboEntry::from(entry);
    proto
        .set_combo(idx, &proto_entry)
        .map_err(|e| ViaError::Hid(e.to_string()))
}

pub fn read_key_overrides(vendor_id: u16, product_id: u16) -> ViaResult<Vec<KeyOverride>> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let counts = proto
        .get_dynamic_entry_counts()
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    let entries = proto
        .get_all_key_overrides(counts.key_override)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(entries.into_iter().map(KeyOverride::from).collect())
}

pub fn write_key_override(
    vendor_id: u16,
    product_id: u16,
    idx: u8,
    entry: &KeyOverride,
) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let proto_entry = ProtoKeyOverrideEntry::from(entry);
    proto
        .set_key_override(idx, &proto_entry)
        .map_err(|e| ViaError::Hid(e.to_string()))
}

/// Probe whether the board is Vial, dynamic-entry capacity, and lighting label.
pub fn probe_capabilities(vendor_id: u16, product_id: u16) -> ViaResult<BoardCapabilities> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);

    let is_vial = matches!(proto.vial_get_keyboard_id(), Ok((v, _)) if v > 0);

    let (tap_dance, combo, key_override) = if is_vial {
        match proto.get_dynamic_entry_counts() {
            Ok(c) => (c.tap_dance, c.combo, c.key_override),
            Err(_) => (0, 0, 0),
        }
    } else {
        (0, 0, 0)
    };

    let lighting = proto.detect_lighting_protocol();
    let lighting_label = lighting.as_ref().map(lighting_proto_label);

    let paint_backend = if probe_openrgb_offset(&device) {
        PaintBackend::OpenRgbOffset
    } else if matches!(lighting, Some(LightingProtocol::VialRgb)) {
        PaintBackend::VialRgbDirect
    } else {
        PaintBackend::None
    };

    Ok(BoardCapabilities {
        is_vial,
        tap_dance,
        combo,
        key_override,
        lighting_label,
        paint_backend,
    })
}

pub fn hid_permission_hint() -> Option<String> {
    match check_hid_permissions() {
        HidAccessStatus::Ok => None,
        other => Some(format!(
            "HID access may be blocked ({other:?}). On Linux, add a udev rule for \
             hidraw (VIA usage page 0xFF60) and re-plug the keyboard."
        )),
    }
}

pub fn discover_devices() -> ViaResult<Vec<DiscoveredDevice>> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let list = discover_keyboards(&api);
    Ok(list
        .into_iter()
        .map(|k| DiscoveredDevice {
            vendor_id: k.vendor_id,
            product_id: k.product_id,
            manufacturer: k.manufacturer,
            product: k.product,
            path: k.path,
        })
        .collect())
}

pub fn find_definition_for(
    defs: &[ViaDefinition],
    vendor_id: u16,
    product_id: u16,
) -> Option<usize> {
    defs.iter()
        .position(|d| d.vendor_id == vendor_id && d.product_id == product_id)
}

fn open_matching(
    api: &hidapi::HidApi,
    vendor_id: u16,
    product_id: u16,
) -> ViaResult<(KeyboardInfo, KeyboardDevice)> {
    let list = discover_keyboards(api);
    let info = list
        .into_iter()
        .find(|k| k.vendor_id == vendor_id && k.product_id == product_id)
        .ok_or_else(|| {
            ViaError::Message(format!(
                "no VIA keyboard matched {:04x}:{:04x}",
                vendor_id, product_id
            ))
        })?;
    let device =
        KeyboardDevice::open(api, info.clone()).map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok((info, device))
}

pub fn read_keymap(vendor_id: u16, product_id: u16, rows: u8, cols: u8) -> ViaResult<KeymapSnapshot> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    let protocol = proto
        .get_protocol_version()
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    let layers = proto
        .get_layer_count()
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    let map = proto
        .read_entire_keymap(layers, rows, cols)
        .map_err(|e| ViaError::Hid(e.to_string()))?;
    Ok(KeymapSnapshot {
        protocol,
        layers,
        map,
    })
}

pub fn set_keycode(
    vendor_id: u16,
    product_id: u16,
    layer: u8,
    row: u8,
    col: u8,
    keycode: u16,
) -> ViaResult<()> {
    let api = hidapi::HidApi::new().map_err(|e| ViaError::Hid(e.to_string()))?;
    let (_info, device) = open_matching(&api, vendor_id, product_id)?;
    let proto = ViaProtocol::new(&device);
    proto
        .set_keycode(layer, row, col, keycode)
        .map_err(|e| ViaError::Hid(e.to_string()))
}

pub fn custom_keycode_value(index: usize) -> u16 {
    QK_KB_0.saturating_add(index as u16)
}

pub fn keycode_short_label(code: u16, customs: &[CustomKeycode]) -> String {
    if let Some((i, c)) = customs
        .iter()
        .enumerate()
        .find(|(i, _)| custom_keycode_value(*i) == code)
    {
        let _ = i;
        return if c.short_name.is_empty() {
            c.name.clone()
        } else {
            c.short_name.clone()
        };
    }
    let kc = Keycode(code);
    let short = kc.short_name();
    if short.starts_with("0x") || short.is_empty() {
        let name = kc.name();
        if name.starts_with("0x") {
            format!("{code:04X}")
        } else {
            name
        }
    } else {
        short
    }
}

pub fn keycode_full_label(code: u16, customs: &[CustomKeycode]) -> String {
    if let Some((i, c)) = customs
        .iter()
        .enumerate()
        .find(|(i, _)| custom_keycode_value(*i) == code)
    {
        let _ = i;
        if c.title.is_empty() {
            format!("{} (0x{code:04X})", c.name)
        } else {
            format!("{} — {} (0x{code:04X})", c.name, c.title)
        }
    } else {
        format!("{} (0x{code:04X})", Keycode(code).name())
    }
}

pub fn picker_entries(customs: &[CustomKeycode]) -> Vec<(String, u16)> {
    let mut out = Vec::new();
    for group in via_protocol::keycode_groups() {
        for kc in group.codes {
            out.push((format!("{} · {}", group.name, kc.name()), kc.0));
        }
    }
    for (i, c) in customs.iter().enumerate() {
        let code = custom_keycode_value(i);
        let label = if c.title.is_empty() {
            format!("Custom · {}", c.name)
        } else {
            format!("Custom · {} ({})", c.name, c.title)
        };
        out.push((label, code));
    }
    out
}

pub fn looks_like_via_definition(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    v.get("vendorId").is_some()
        && v.get("productId").is_some()
        && v.get("matrix").is_some()
        && v.pointer("/layouts/keymap").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_r75_definition() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("via-definitions/r75-qmk.json");
        let def = load_definition(&path).expect("load r75");
        assert_eq!(def.vendor_id, 0x342d);
        assert_eq!(def.product_id, 0xe484);
        assert_eq!(def.rows, 6);
        assert_eq!(def.cols, 15);
        assert!(!def.layout.keys.is_empty());
        assert_eq!(def.custom_keycodes.len(), 7);
        assert!(def.lighting.declared);
        assert!(def.lighting.has_brightness);
        assert!(def.lighting.has_speed);
        assert!(def.lighting.has_color);
        assert!(def.lighting.effects.len() > 40);
        assert_eq!(def.lighting.effects[1].0, "Solid Color");
    }

    /// Smoke: definition JSON without menus still parses (Vial-fetched style).
    #[test]
    fn parse_definition_without_lighting_menus() {
        let json = r#"{
            "name": "Smoke Board",
            "vendorId": "0x1234",
            "productId": "0x5678",
            "matrix": { "rows": 2, "cols": 3 },
            "layouts": {
                "keymap": [
                    ["0,0", "0,1", "0,2"],
                    ["1,0", "1,1", "1,2"]
                ]
            },
            "customKeycodes": [
                { "name": "FOO", "title": "Foo", "shortName": "F" }
            ]
        }"#;
        let def = parse_definition_json(json, PathBuf::from("smoke.json")).expect("parse");
        assert_eq!(def.name, "Smoke Board");
        assert_eq!(def.vendor_id, 0x1234);
        assert_eq!(def.product_id, 0x5678);
        assert_eq!(def.rows, 2);
        assert_eq!(def.cols, 3);
        assert_eq!(def.layout.keys.len(), 6);
        assert_eq!(def.custom_keycodes.len(), 1);
        assert!(!def.lighting.declared);
        assert!(def.lighting.effects.is_empty());
    }

    #[test]
    fn parse_definition_with_override_ids() {
        let json = r#"{
            "name": "No Ids",
            "matrix": { "rows": 1, "cols": 1 },
            "layouts": { "keymap": [["0,0"]] }
        }"#;
        let def = parse_definition_json_with_ids(
            json,
            PathBuf::from("vial-fetched.json"),
            Some((0x342d, 0xe484)),
        )
        .expect("parse with override ids");
        assert_eq!(def.vendor_id, 0x342d);
        assert_eq!(def.product_id, 0xe484);
        assert_eq!(def.rows, 1);
        assert_eq!(def.cols, 1);
    }

    #[test]
    fn dynamic_entry_roundtrip_structs() {
        let td = TapDance {
            on_tap: 0x0004,
            on_hold: 0x00E0,
            on_double_tap: 0x0005,
            on_tap_hold: 0,
            tapping_term: 200,
        };
        let proto = ProtoTapDanceEntry::from(&td);
        assert_eq!(TapDance::from(proto), td);

        let combo = Combo {
            input: [0x0004, 0x0005, 0, 0],
            output: 0x001B,
        };
        assert_eq!(Combo::from(ProtoComboEntry::from(&combo)), combo);

        let ko = KeyOverride {
            trigger: 0x0004,
            replacement: 0x0005,
            layers: 0xFFFF,
            trigger_mods: 0x02,
            negative_mod_mask: 0,
            suppressed_mods: 0x02,
            options: 0x80,
        };
        assert_eq!(KeyOverride::from(ProtoKeyOverrideEntry::from(&ko)), ko);
    }
}
