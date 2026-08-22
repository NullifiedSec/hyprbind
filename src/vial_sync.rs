//! Vial-first keyboard discovery and live hardware state snapshots.
//!
//! Vial-capable keyboards are self-describing, so their onboard definition and
//! current dynamic keymap are the source of truth. The GTK visualizer can build
//! directly from this model and then overlay Hyprland binds on top.

use crate::experimental::via::{self, DiscoveredDevice, KeymapSnapshot, ViaDefinition, ViaResult};

#[derive(Debug, Clone)]
pub struct VialBoardSnapshot {
    pub device: DiscoveredDevice,
    pub definition: ViaDefinition,
    pub keymap: KeymapSnapshot,
}

impl VialBoardSnapshot {
    pub fn layer_count(&self) -> u8 {
        self.keymap.layers
    }

    pub fn keycode_at(&self, layer: u8, row: u8, col: u8) -> Option<u16> {
        self.keymap
            .map
            .get(layer as usize)
            .and_then(|rows| rows.get(row as usize))
            .and_then(|cols| cols.get(col as usize))
            .copied()
    }

    pub fn physical_keys(&self) -> &[via_protocol::KeyPosition] {
        &self.definition.layout.keys
    }

    pub fn display_name(&self) -> String {
        let product = self.device.product.trim();
        let name = if product.is_empty() {
            self.definition.name.as_str()
        } else {
            product
        };
        format!(
            "{} ({:04x}:{:04x})",
            name, self.device.vendor_id, self.device.product_id
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct VialDiscovery {
    pub boards: Vec<VialBoardSnapshot>,
    /// Per-device soft failures. One broken/inaccessible keyboard should not
    /// prevent other Vial boards from appearing.
    pub diagnostics: Vec<String>,
}

/// Discover every accessible Vial keyboard and capture the state required to
/// render its real physical layout and current hardware keymap.
pub fn discover() -> ViaResult<VialDiscovery> {
    let devices = via::discover_devices()?;
    let mut result = VialDiscovery::default();

    for device in devices {
        let vid = device.vendor_id;
        let pid = device.product_id;
        let label = device_label(&device);

        match via::detect_vial(vid, pid) {
            Ok(Some(_)) => {}
            Ok(None) => continue,
            Err(err) => {
                result
                    .diagnostics
                    .push(format!("{label}: Vial probe failed: {err}"));
                continue;
            }
        }

        let definition = match via::fetch_vial_definition(vid, pid) {
            Ok(definition) => definition,
            Err(err) => {
                result
                    .diagnostics
                    .push(format!("{label}: definition fetch failed: {err}"));
                continue;
            }
        };

        let keymap = match via::read_keymap(vid, pid, definition.rows, definition.cols) {
            Ok(keymap) => keymap,
            Err(err) => {
                result
                    .diagnostics
                    .push(format!("{label}: keymap read failed: {err}"));
                continue;
            }
        };

        result.boards.push(VialBoardSnapshot {
            device,
            definition,
            keymap,
        });
    }

    Ok(result)
}

fn device_label(device: &DiscoveredDevice) -> String {
    let product = device.product.trim();
    let name = if product.is_empty() {
        "keyboard"
    } else {
        product
    };
    format!("{name} ({:04x}:{:04x})", device.vendor_id, device.product_id)
}

/// Human-readable dump used by the CLI and useful while bringing up the GTK
/// visualizer on new keyboards.
pub fn report() -> String {
    match discover() {
        Ok(discovery) => {
            let mut out = String::new();
            if discovery.boards.is_empty() {
                out.push_str("No accessible Vial keyboards detected.\n");
            }

            for board in &discovery.boards {
                out.push_str(&format!("{}\n", board.display_name()));
                out.push_str(&format!(
                    "  definition: {}\n  matrix: {}x{}\n  physical keys: {}\n  layers: {}\n  protocol: {}\n",
                    board.definition.name,
                    board.definition.rows,
                    board.definition.cols,
                    board.physical_keys().len(),
                    board.layer_count(),
                    board.keymap.protocol,
                ));
            }

            if !discovery.diagnostics.is_empty() {
                out.push_str("diagnostics:\n");
                for diagnostic in &discovery.diagnostics {
                    out.push_str(&format!("  - {diagnostic}\n"));
                }
            }
            out
        }
        Err(err) => format!("Vial discovery failed: {err}\n"),
    }
}
