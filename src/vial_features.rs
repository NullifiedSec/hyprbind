//! Remaining first-class Vial protocol surfaces used by Hyprbinds.
//!
//! Wire layouts mirror the pinned official Vial GUI protocol implementation:
//! unlock/security commands, safe matrix polling, dynamic Alt Repeat entries,
//! Vial encoder commands, and the advanced macro bytecode format.

use crate::experimental::via;
use crate::vial_native;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use via_protocol::device::{discover_keyboards, KeyboardDevice};
use via_protocol::{ViaCommand, ViaCommandId, ViaProtocol};

const VIAL_GET_ENCODER: u8 = 0x03;
const VIAL_SET_ENCODER: u8 = 0x04;
const VIAL_GET_UNLOCK_STATUS: u8 = 0x05;
const VIAL_UNLOCK_START: u8 = 0x06;
const VIAL_UNLOCK_POLL: u8 = 0x07;
const VIAL_LOCK: u8 = 0x08;
const VIAL_DYNAMIC_ENTRY_OP: u8 = 0x0D;
const VIAL_ALT_REPEAT_GET: u8 = 0x07;
const VIAL_ALT_REPEAT_SET: u8 = 0x08;
const VIA_SWITCH_MATRIX_STATE: u8 = 0x03;

const VIAL_PROTOCOL_ADVANCED_MACROS: u32 = 2;
pub const VIAL_PROTOCOL_MATRIX_TESTER: u32 = 3;
pub const VIAL_PROTOCOL_DYNAMIC: u32 = 4;
pub const VIAL_PROTOCOL_QMK_SETTINGS: u32 = 4;
pub const VIAL_PROTOCOL_EXT_MACROS: u32 = 5;

const SS_QMK_PREFIX: u8 = 1;
const SS_TAP_CODE: u8 = 1;
const SS_DOWN_CODE: u8 = 2;
const SS_UP_CODE: u8 = 3;
const SS_DELAY_CODE: u8 = 4;
const VIAL_MACRO_EXT_TAP: u8 = 5;
const VIAL_MACRO_EXT_DOWN: u8 = 6;
const VIAL_MACRO_EXT_UP: u8 = 7;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AltRepeatEntry {
    pub keycode: u16,
    pub alt_keycode: u16,
    pub allowed_mods: u8,
    /// bit 0 default-to-alt, bit 1 bidirectional, bit 2 ignore handedness,
    /// bit 3 enabled.
    pub options: u8,
}

impl AltRepeatEntry {
    pub fn enabled(&self) -> bool {
        self.options & (1 << 3) != 0
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            self.options |= 1 << 3;
        } else {
            self.options &= !(1 << 3);
        }
    }

    pub fn default_to_alt(&self) -> bool {
        self.options & 1 != 0
    }

    pub fn bidirectional(&self) -> bool {
        self.options & (1 << 1) != 0
    }

    pub fn ignore_mod_handedness(&self) -> bool {
        self.options & (1 << 2) != 0
    }

    fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 6 {
            return Err("short Alt Repeat entry response".into());
        }
        Ok(Self {
            keycode: u16::from_le_bytes([data[0], data[1]]),
            alt_keycode: u16::from_le_bytes([data[2], data[3]]),
            allowed_mods: data[4],
            options: data[5],
        })
    }

    fn to_bytes(&self) -> [u8; 6] {
        let mut out = [0u8; 6];
        out[0..2].copy_from_slice(&self.keycode.to_le_bytes());
        out[2..4].copy_from_slice(&self.alt_keycode.to_le_bytes());
        out[4] = self.allowed_mods;
        out[5] = self.options;
        out
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockStatus {
    pub unlocked: bool,
    pub in_progress: bool,
    pub required_keys: Vec<(u8, u8)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockProgress {
    pub unlocked: bool,
    pub counter: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixSnapshot {
    pub rows: u8,
    pub cols: u8,
    pub pressed: Vec<Vec<bool>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderPair {
    pub clockwise: u16,
    pub counter_clockwise: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroAction {
    Text(String),
    Tap(Vec<u16>),
    Down(Vec<u16>),
    Up(Vec<u16>),
    Delay(u16),
    /// Unknown bytes are preserved instead of being silently discarded.
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroBank {
    pub vial_protocol: u32,
    pub count: u8,
    pub buffer_size: u16,
    pub macros: Vec<Vec<MacroAction>>,
}

pub fn read_alt_repeat_entries(vendor_id: u16, product_id: u16) -> Result<Vec<AltRepeatEntry>, String> {
    with_protocol(vendor_id, product_id, |proto, device| {
        let counts = proto.get_dynamic_entry_counts().map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(counts.alt_repeat as usize);
        for idx in 0..counts.alt_repeat {
            let response = device
                .send_command(&ViaCommand::with_data(
                    ViaCommandId::VialPrefix,
                    &[VIAL_DYNAMIC_ENTRY_OP, VIAL_ALT_REPEAT_GET, idx],
                ))
                .map_err(|e| e.to_string())?;
            // Dynamic-entry GET responses put status in byte 0.
            if response.first().copied().unwrap_or(0xFF) != 0 {
                return Err(format!("Alt Repeat slot {idx} read failed with status {}", response[0]));
            }
            out.push(AltRepeatEntry::from_bytes(&response[1..7])?);
        }
        Ok(out)
    })
}

pub fn write_alt_repeat_entry(
    vendor_id: u16,
    product_id: u16,
    index: u8,
    entry: &AltRepeatEntry,
) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        let mut payload = vec![VIAL_DYNAMIC_ENTRY_OP, VIAL_ALT_REPEAT_SET, index];
        payload.extend_from_slice(&entry.to_bytes());
        let response = device
            .send_command(&ViaCommand::with_data(ViaCommandId::VialPrefix, &payload))
            .map_err(|e| e.to_string())?;
        if response.first().copied().unwrap_or(0) == 0xFF {
            return Err("Alt Repeat command is not supported by this firmware".into());
        }
        Ok(())
    })
}

pub fn read_unlock_status(vendor_id: u16, product_id: u16) -> Result<UnlockStatus, String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        let response = vial_simple(device, VIAL_GET_UNLOCK_STATUS, &[])?;
        let unlocked = response.first().copied().unwrap_or(0) == 1;
        let in_progress = response.get(1).copied().unwrap_or(0) != 0;
        let mut required_keys = Vec::new();
        for pair in response.get(2..).unwrap_or_default().chunks_exact(2).take(15) {
            if pair[0] != 0xFF && pair[1] != 0xFF {
                required_keys.push((pair[0], pair[1]));
            }
        }
        Ok(UnlockStatus {
            unlocked,
            in_progress,
            required_keys,
        })
    })
}

pub fn unlock_start(vendor_id: u16, product_id: u16) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        vial_simple(device, VIAL_UNLOCK_START, &[])?;
        Ok(())
    })
}

pub fn unlock_poll(vendor_id: u16, product_id: u16) -> Result<UnlockProgress, String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        let response = vial_simple(device, VIAL_UNLOCK_POLL, &[])?;
        Ok(UnlockProgress {
            unlocked: response.first().copied().unwrap_or(0) == 1,
            counter: response.get(2).copied().unwrap_or(0),
        })
    })
}

pub fn lock(vendor_id: u16, product_id: u16) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        vial_simple(device, VIAL_LOCK, &[])?;
        Ok(())
    })
}

/// Read the safe switch-matrix state exposed through VIA keyboard value 0x03.
pub fn matrix_poll(
    vendor_id: u16,
    product_id: u16,
    rows: u8,
    cols: u8,
) -> Result<MatrixSnapshot, String> {
    if rows == 0 || cols == 0 {
        return Err("matrix dimensions must be non-zero".into());
    }
    let row_size = (cols as usize + 7) / 8;
    if row_size * rows as usize > 28 {
        return Err("matrix is too large for VIA's 28-byte value payload".into());
    }

    with_protocol(vendor_id, product_id, |_proto, device| {
        let response = device
            .send_command(&ViaCommand::with_data(
                ViaCommandId::GetKeyboardValue,
                &[VIA_SWITCH_MATRIX_STATE],
            ))
            .map_err(|e| e.to_string())?;
        if response.first().copied().unwrap_or(0) == 0xFF {
            return Err("matrix tester is not supported by this firmware".into());
        }

        let payload = response.get(2..).unwrap_or_default();
        let expected = row_size * rows as usize;
        if payload.len() < expected {
            return Err(format!(
                "short matrix response: expected {expected} data bytes, got {}",
                payload.len()
            ));
        }

        let mut pressed = vec![vec![false; cols as usize]; rows as usize];
        for row in 0..rows as usize {
            let row_data = &payload[row * row_size..(row + 1) * row_size];
            for col in 0..cols as usize {
                // Vial's matrix tester stores each row most-significant byte first.
                let byte_index = row_data.len() - 1 - (col / 8);
                let bit = col % 8;
                pressed[row][col] = ((row_data[byte_index] >> bit) & 1) != 0;
            }
        }

        Ok(MatrixSnapshot { rows, cols, pressed })
    })
}

pub fn encoder_count(vendor_id: u16, product_id: u16) -> Result<u8, String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        let json = proto.vial_get_definition().map_err(|e| e.to_string())?;
        encoder_count_from_json(&json)
    })
}

pub fn read_vial_encoder(
    vendor_id: u16,
    product_id: u16,
    layer: u8,
    encoder: u8,
) -> Result<EncoderPair, String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        let response = vial_simple(device, VIAL_GET_ENCODER, &[layer, encoder])?;
        if response.len() < 4 {
            return Err("short Vial encoder response".into());
        }
        Ok(EncoderPair {
            counter_clockwise: u16::from_be_bytes([response[0], response[1]]),
            clockwise: u16::from_be_bytes([response[2], response[3]]),
        })
    })
}

pub fn write_vial_encoder(
    vendor_id: u16,
    product_id: u16,
    layer: u8,
    encoder: u8,
    clockwise: bool,
    keycode: u16,
) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        let direction = if clockwise { 1 } else { 0 };
        vial_simple(
            device,
            VIAL_SET_ENCODER,
            &[
                layer,
                encoder,
                direction,
                (keycode >> 8) as u8,
                keycode as u8,
            ],
        )?;
        Ok(())
    })
}

pub fn read_macro_bank(vendor_id: u16, product_id: u16) -> Result<MacroBank, String> {
    let snapshot = vial_native::read_macros(vendor_id, product_id)?;
    let vial_protocol = via::detect_vial(vendor_id, product_id)
        .map_err(|e| e.to_string())?
        .map(|v| v.protocol_version)
        .unwrap_or(0);

    let mut chunks = snapshot.buffer.split(|b| *b == 0);
    let mut macros = Vec::with_capacity(snapshot.count as usize);
    for _ in 0..snapshot.count {
        let raw = chunks.next().unwrap_or_default();
        macros.push(decode_macro(raw, vial_protocol));
    }

    Ok(MacroBank {
        vial_protocol,
        count: snapshot.count,
        buffer_size: snapshot.buffer_size,
        macros,
    })
}

pub fn write_macro_bank(vendor_id: u16, product_id: u16, bank: &MacroBank) -> Result<(), String> {
    if bank.macros.len() != bank.count as usize {
        return Err(format!(
            "expected {} macro slots, got {}",
            bank.count,
            bank.macros.len()
        ));
    }

    let mut encoded = Vec::new();
    for actions in &bank.macros {
        encoded.extend_from_slice(&encode_macro(actions, bank.vial_protocol)?);
        encoded.push(0);
    }

    let size = bank.buffer_size as usize;
    if encoded.len() > size {
        return Err(format!(
            "macro program requires {} bytes but keyboard has {}",
            encoded.len(), size
        ));
    }
    encoded.resize(size, 0);
    vial_native::write_macro_buffer(vendor_id, product_id, &encoded)
}

pub fn macro_to_json(actions: &[MacroAction]) -> Result<String, String> {
    serde_json::to_string_pretty(actions).map_err(|e| e.to_string())
}

pub fn macro_from_json(text: &str) -> Result<Vec<MacroAction>, String> {
    serde_json::from_str(text).map_err(|e| format!("invalid macro action JSON: {e}"))
}

pub fn qmk_setting_get(vendor_id: u16, product_id: u16, id: u16) -> Result<Vec<u8>, String> {
    vial_native::qmk_setting_get(vendor_id, product_id, id)
}

pub fn qmk_setting_set(
    vendor_id: u16,
    product_id: u16,
    id: u16,
    value: &[u8],
) -> Result<(), String> {
    vial_native::qmk_setting_set(vendor_id, product_id, id, value)
}

pub fn qmk_settings_reset(vendor_id: u16, product_id: u16) -> Result<(), String> {
    vial_native::qmk_settings_reset(vendor_id, product_id)
}

pub fn decode_macro(data: &[u8], vial_protocol: u32) -> Vec<MacroAction> {
    let mut out = Vec::new();
    let mut text = Vec::new();
    let mut i = 0usize;

    let flush_text = |out: &mut Vec<MacroAction>, text: &mut Vec<u8>| {
        if !text.is_empty() {
            out.push(MacroAction::Text(String::from_utf8_lossy(text).into_owned()));
            text.clear();
        }
    };

    while i < data.len() {
        if vial_protocol >= VIAL_PROTOCOL_ADVANCED_MACROS && data[i] == SS_QMK_PREFIX {
            if i + 1 >= data.len() {
                text.push(data[i]);
                break;
            }
            flush_text(&mut out, &mut text);
            let op = data[i + 1];
            match op {
                SS_TAP_CODE | SS_DOWN_CODE | SS_UP_CODE => {
                    if i + 2 >= data.len() {
                        out.push(MacroAction::Raw(data[i..].to_vec()));
                        break;
                    }
                    push_sequence_action(&mut out, op, data[i + 2] as u16);
                    i += 3;
                }
                VIAL_MACRO_EXT_TAP | VIAL_MACRO_EXT_DOWN | VIAL_MACRO_EXT_UP => {
                    if i + 3 >= data.len() {
                        out.push(MacroAction::Raw(data[i..].to_vec()));
                        break;
                    }
                    let mut kc = u16::from_le_bytes([data[i + 2], data[i + 3]]);
                    if kc > 0xFF00 {
                        kc = (kc & 0x00FF) << 8;
                    }
                    let base = match op {
                        VIAL_MACRO_EXT_TAP => SS_TAP_CODE,
                        VIAL_MACRO_EXT_DOWN => SS_DOWN_CODE,
                        _ => SS_UP_CODE,
                    };
                    push_sequence_action(&mut out, base, kc);
                    i += 4;
                }
                SS_DELAY_CODE => {
                    if i + 3 >= data.len() {
                        out.push(MacroAction::Raw(data[i..].to_vec()));
                        break;
                    }
                    let lo = data[i + 2].saturating_sub(1) as u16;
                    let hi = data[i + 3].saturating_sub(1) as u16;
                    out.push(MacroAction::Delay(lo + hi * 255));
                    i += 4;
                }
                _ => {
                    out.push(MacroAction::Raw(vec![data[i], data[i + 1]]));
                    i += 2;
                }
            }
            continue;
        }

        if vial_protocol < VIAL_PROTOCOL_ADVANCED_MACROS
            && matches!(data[i], SS_TAP_CODE | SS_DOWN_CODE | SS_UP_CODE)
        {
            if i + 1 >= data.len() {
                text.push(data[i]);
                break;
            }
            flush_text(&mut out, &mut text);
            push_sequence_action(&mut out, data[i], data[i + 1] as u16);
            i += 2;
            continue;
        }

        text.push(data[i]);
        i += 1;
    }

    flush_text(&mut out, &mut text);
    out
}

pub fn encode_macro(actions: &[MacroAction], vial_protocol: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for action in actions {
        match action {
            MacroAction::Text(text) => {
                if text.as_bytes().contains(&0) {
                    return Err("macro text cannot contain NUL".into());
                }
                out.extend_from_slice(text.as_bytes());
            }
            MacroAction::Tap(keys) => encode_sequence(&mut out, keys, SS_TAP_CODE, VIAL_MACRO_EXT_TAP, vial_protocol)?,
            MacroAction::Down(keys) => encode_sequence(&mut out, keys, SS_DOWN_CODE, VIAL_MACRO_EXT_DOWN, vial_protocol)?,
            MacroAction::Up(keys) => encode_sequence(&mut out, keys, SS_UP_CODE, VIAL_MACRO_EXT_UP, vial_protocol)?,
            MacroAction::Delay(delay) => {
                if vial_protocol < VIAL_PROTOCOL_ADVANCED_MACROS {
                    return Err("delays require Vial protocol >= 2".into());
                }
                if *delay > 65_024 {
                    return Err("macro delay exceeds Vial's 65024 ms encoding limit".into());
                }
                out.extend_from_slice(&[
                    SS_QMK_PREFIX,
                    SS_DELAY_CODE,
                    (*delay % 255) as u8 + 1,
                    (*delay / 255) as u8 + 1,
                ]);
            }
            MacroAction::Raw(bytes) => out.extend_from_slice(bytes),
        }
    }
    Ok(out)
}

fn encode_sequence(
    out: &mut Vec<u8>,
    keys: &[u16],
    basic_op: u8,
    extended_op: u8,
    vial_protocol: u32,
) -> Result<(), String> {
    for &keycode in keys {
        if vial_protocol >= VIAL_PROTOCOL_ADVANCED_MACROS {
            out.push(SS_QMK_PREFIX);
        }

        if keycode < 256 {
            out.push(basic_op);
            out.push(keycode as u8);
        } else {
            if vial_protocol < VIAL_PROTOCOL_ADVANCED_MACROS {
                return Err(format!(
                    "keycode 0x{keycode:04X} requires Vial advanced macros"
                ));
            }
            out.push(extended_op);
            let encoded = if keycode & 0x00FF == 0 {
                0xFF00 | (keycode >> 8)
            } else {
                keycode
            };
            out.extend_from_slice(&encoded.to_le_bytes());
        }
    }
    Ok(())
}

fn push_sequence_action(out: &mut Vec<MacroAction>, op: u8, keycode: u16) {
    let same = |action: &MacroAction| match (op, action) {
        (SS_TAP_CODE, MacroAction::Tap(_)) => true,
        (SS_DOWN_CODE, MacroAction::Down(_)) => true,
        (SS_UP_CODE, MacroAction::Up(_)) => true,
        _ => false,
    };

    if let Some(last) = out.last_mut().filter(|a| same(a)) {
        match last {
            MacroAction::Tap(keys) | MacroAction::Down(keys) | MacroAction::Up(keys) => {
                keys.push(keycode);
                return;
            }
            _ => {}
        }
    }

    out.push(match op {
        SS_TAP_CODE => MacroAction::Tap(vec![keycode]),
        SS_DOWN_CODE => MacroAction::Down(vec![keycode]),
        _ => MacroAction::Up(vec![keycode]),
    });
}

fn encoder_count_from_json(text: &str) -> Result<u8, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let keymap = root
        .get("layouts")
        .and_then(|v| v.get("keymap"))
        .ok_or_else(|| "Vial definition has no layouts.keymap".to_string())?;
    let mut max_index: Option<u8> = None;
    walk_encoder_legends(keymap, &mut max_index);
    Ok(max_index.map(|v| v.saturating_add(1)).unwrap_or(0))
}

fn walk_encoder_legends(value: &Value, max_index: &mut Option<u8>) {
    match value {
        Value::Array(items) => {
            for item in items {
                walk_encoder_legends(item, max_index);
            }
        }
        Value::String(legend) => {
            if !legend.lines().any(|line| line.trim() == "e") {
                return;
            }
            let first = legend.lines().next().unwrap_or_default().trim();
            if let Some((idx, _direction)) = first.split_once(',') {
                if let Ok(idx) = idx.trim().parse::<u8>() {
                    *max_index = Some(max_index.map(|m| m.max(idx)).unwrap_or(idx));
                }
            }
        }
        _ => {}
    }
}

fn vial_simple(device: &KeyboardDevice, subcommand: u8, rest: &[u8]) -> Result<Vec<u8>, String> {
    let mut payload = Vec::with_capacity(rest.len() + 1);
    payload.push(subcommand);
    payload.extend_from_slice(rest);
    device
        .send_command(&ViaCommand::with_data(ViaCommandId::VialPrefix, &payload))
        .map_err(|e| e.to_string())
}

fn with_protocol<T>(
    vendor_id: u16,
    product_id: u16,
    f: impl FnOnce(&ViaProtocol<'_>, &KeyboardDevice) -> Result<T, String>,
) -> Result<T, String> {
    let api = hidapi::HidApi::new().map_err(|e| e.to_string())?;
    let info = discover_keyboards(&api)
        .into_iter()
        .find(|info| info.vendor_id == vendor_id && info.product_id == product_id)
        .ok_or_else(|| format!("VIA/Vial keyboard {vendor_id:04x}:{product_id:04x} not found"))?;
    let device = KeyboardDevice::open(&api, info).map_err(|e| e.to_string())?;
    let proto = ViaProtocol::new(&device);
    f(&proto, &device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_macro_round_trip() {
        let actions = vec![
            MacroAction::Text("hello".into()),
            MacroAction::Tap(vec![0x04, 0x1234]),
            MacroAction::Down(vec![0xE1]),
            MacroAction::Delay(500),
            MacroAction::Up(vec![0xE1]),
        ];
        let encoded = encode_macro(&actions, 5).unwrap();
        let decoded = decode_macro(&encoded, 5);
        assert_eq!(decoded, actions);
    }

    #[test]
    fn legacy_macro_rejects_extended_features() {
        assert!(encode_macro(&[MacroAction::Delay(10)], 1).is_err());
        assert!(encode_macro(&[MacroAction::Tap(vec![0x1234])], 1).is_err());
    }

    #[test]
    fn alt_repeat_option_bits_are_stable() {
        let mut entry = AltRepeatEntry::default();
        entry.options = 0b0111;
        entry.set_enabled(true);
        assert!(entry.enabled());
        assert!(entry.default_to_alt());
        assert!(entry.bidirectional());
        assert!(entry.ignore_mod_handedness());
        assert_eq!(entry.to_bytes()[5], 0b1111);
    }
}
