//! Native VIA/Vial protocol helpers that are not yet exposed by the legacy
//! experimental VIA module.
//!
//! The wire format for dynamic macro buffers follows QMK/VIA: offset (u16 BE),
//! size (u8, max 28), then payload. Macro writes deliberately use QMK's
//! invalid-buffer sentinel (last byte = 0xff while writing, 0x00 when complete)
//! so an interrupted transfer cannot be interpreted as a valid macro table.

use via_protocol::device::{discover_keyboards, KeyboardDevice};
use via_protocol::{ViaCommand, ViaCommandId, ViaProtocol};

const MAX_CHUNK: usize = 28;

#[derive(Debug, Clone, Default)]
pub struct MacroSnapshot {
    pub count: u8,
    pub buffer_size: u16,
    pub buffer: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct NativeProtocolSnapshot {
    pub macro_count: Option<u8>,
    pub macro_buffer_size: Option<u16>,
    pub qmk_setting_ids: Vec<u16>,
    pub qmk_settings_supported: bool,
}

pub fn inspect(vendor_id: u16, product_id: u16) -> Result<NativeProtocolSnapshot, String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        let macro_count = proto.get_macro_count().ok();
        let macro_buffer_size = proto.get_macro_buffer_size().ok();
        let qmk_settings_supported = proto.has_qmk_settings();
        let qmk_setting_ids = if qmk_settings_supported {
            proto.qmk_settings_query().unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(NativeProtocolSnapshot {
            macro_count,
            macro_buffer_size,
            qmk_setting_ids,
            qmk_settings_supported,
        })
    })
}

pub fn read_macros(vendor_id: u16, product_id: u16) -> Result<MacroSnapshot, String> {
    with_protocol(vendor_id, product_id, |proto, device| {
        let count = proto.get_macro_count().map_err(|e| e.to_string())?;
        let buffer_size = proto.get_macro_buffer_size().map_err(|e| e.to_string())?;
        let mut buffer = Vec::with_capacity(buffer_size as usize);
        let mut offset = 0usize;

        while offset < buffer_size as usize {
            let size = MAX_CHUNK.min(buffer_size as usize - offset);
            let command = ViaCommand {
                id: ViaCommandId::DynamicKeymapMacroGetBuffer,
                data: vec![(offset >> 8) as u8, offset as u8, size as u8],
            };
            let response = device.send_command(&command).map_err(|e| e.to_string())?;
            let start = 4;
            let end = start + size;
            buffer.extend_from_slice(&response[start..end]);
            offset += size;
        }

        Ok(MacroSnapshot {
            count,
            buffer_size,
            buffer,
        })
    })
}

/// Replace the complete macro buffer using QMK's validity-sentinel protocol.
///
/// `buffer` must exactly match the board-reported macro buffer size. The final
/// byte is reserved as the validity flag and will be forced to zero after all
/// other bytes have been written successfully.
pub fn write_macro_buffer(
    vendor_id: u16,
    product_id: u16,
    buffer: &[u8],
) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |proto, device| {
        let expected = proto.get_macro_buffer_size().map_err(|e| e.to_string())? as usize;
        if buffer.len() != expected {
            return Err(format!(
                "macro buffer length mismatch: keyboard expects {expected} bytes, got {}",
                buffer.len()
            ));
        }
        if expected == 0 {
            return Ok(());
        }

        // Mark the EEPROM macro region invalid before changing its contents.
        write_macro_chunk(device, expected - 1, &[0xff])?;

        // Write all bytes except the sentinel. If any write fails, the sentinel
        // remains non-zero and firmware treats the macro table as invalid.
        let payload_len = expected - 1;
        let mut offset = 0usize;
        while offset < payload_len {
            let end = (offset + MAX_CHUNK).min(payload_len);
            write_macro_chunk(device, offset, &buffer[offset..end])?;
            offset = end;
        }

        // Commit: valid macro buffers must end with NUL.
        write_macro_chunk(device, expected - 1, &[0x00])?;
        Ok(())
    })
}

pub fn reset_macros(vendor_id: u16, product_id: u16) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |_proto, device| {
        let command = ViaCommand {
            id: ViaCommandId::DynamicKeymapMacroReset,
            data: Vec::new(),
        };
        device.send_command(&command).map_err(|e| e.to_string())?;
        Ok(())
    })
}

pub fn get_encoder(
    vendor_id: u16,
    product_id: u16,
    layer: u8,
    encoder: u8,
    clockwise: bool,
) -> Result<u16, String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        proto
            .get_encoder(layer, encoder, clockwise)
            .map_err(|e| e.to_string())
    })
}

pub fn set_encoder(
    vendor_id: u16,
    product_id: u16,
    layer: u8,
    encoder: u8,
    clockwise: bool,
    keycode: u16,
) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        proto
            .set_encoder(layer, encoder, clockwise, keycode)
            .map_err(|e| e.to_string())
    })
}

pub fn qmk_setting_get(
    vendor_id: u16,
    product_id: u16,
    setting_id: u16,
) -> Result<Vec<u8>, String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        proto
            .qmk_settings_get(setting_id)
            .map_err(|e| e.to_string())
    })
}

pub fn qmk_setting_set(
    vendor_id: u16,
    product_id: u16,
    setting_id: u16,
    value: &[u8],
) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        proto
            .qmk_settings_set(setting_id, value)
            .map_err(|e| e.to_string())
    })
}

pub fn qmk_settings_reset(vendor_id: u16, product_id: u16) -> Result<(), String> {
    with_protocol(vendor_id, product_id, |proto, _device| {
        proto.qmk_settings_reset().map_err(|e| e.to_string())
    })
}

fn write_macro_chunk(device: &KeyboardDevice, offset: usize, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_CHUNK {
        return Err(format!("macro write chunk exceeds {MAX_CHUNK} bytes"));
    }
    if offset > u16::MAX as usize {
        return Err("macro buffer offset exceeds VIA protocol range".into());
    }

    let mut data = Vec::with_capacity(3 + bytes.len());
    data.push((offset >> 8) as u8);
    data.push(offset as u8);
    data.push(bytes.len() as u8);
    data.extend_from_slice(bytes);
    let command = ViaCommand {
        id: ViaCommandId::DynamicKeymapMacroSetBuffer,
        data,
    };
    device.send_command(&command).map_err(|e| e.to_string())?;
    Ok(())
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
        .ok_or_else(|| format!("VIA/Vial keyboard {:04x}:{:04x} not found", vendor_id, product_id))?;
    let device = KeyboardDevice::open(&api, info).map_err(|e| e.to_string())?;
    let proto = ViaProtocol::new(&device);
    f(&proto, &device)
}
