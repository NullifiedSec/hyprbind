# Third-Party Notices and Credits

Hyprbinds is GPL-2.0 software. This project interoperates with and, where noted, reimplements protocol behavior documented or implemented by the following open-source projects.

## Vial GUI

- Project: `vial-kb/vial-gui`
- Website: https://get.vial.today/
- Source: https://github.com/vial-kb/vial-gui
- License: GNU General Public License v2.0
- Pinned protocol/reference revision: `aef8222a2d0429a183b2ed692d5f9efcfd383f08`

Vial is the canonical upstream compatibility target for Vial firmware configuration. Hyprbinds' native Rust implementation interoperates with Vial-capable keyboards while adding Hyprland-aware physical-key and bind correlation.

The native security/unlock flow, safe switch-matrix interpretation, Vial-native encoder packet layout, Alt Repeat entry layout/options, advanced macro action encoding, and QMK-settings metadata were checked against the pinned official Vial implementation. Hyprbinds implements those surfaces in Rust/GTK rather than embedding the Python/Qt application.

When `scripts/sync-vial-upstream.sh` is used, an unmodified checkout of the official Vial GUI is placed under `third_party/vial-gui/`. Its original copyright notices and `COPYING` file remain authoritative for that checkout.

## viar / via-protocol

- Project: `mikkurogue/viar`
- Source: https://github.com/mikkurogue/viar
- Component used: `crates/via-protocol`
- License: MIT
- Copyright: Copyright (c) 2026 Mikku
- Pinned revision: `f7d90d9ddfba108e77577ac228f62f8c1118995e`
- Preserved license text: `LICENSES/viar-MIT.txt`

Hyprbinds uses the Rust `via-protocol` crate as its low-level VIA/Vial HID protocol backend. The feature branch intentionally pins an upstream Git revision newer than the crates.io `0.1.0` snapshot so native dynamic entries, encoder helpers, lighting, and QMK-settings protocol support are available. Hyprbinds-specific integration, security/matrix extensions, GTK UI, Hyprland correlation, hardware-action classification, and macro-buffer transaction logic live in this repository.

## QMK Firmware

Vial firmware is built on QMK and the dynamic-keymap/keycode concepts exposed here come from the QMK ecosystem.

- Project: QMK Firmware
- Source: https://github.com/qmk/qmk_firmware
- Website: https://qmk.fm/

The native macro-buffer writer follows QMK's dynamic macro validity convention: the final macro-buffer byte is marked non-zero before a write and is committed back to NUL only after the rest of the transfer succeeds. This prevents an interrupted update from being treated as a valid macro table.

Refer to QMK's upstream repository for its current licensing and component-specific notices.

## VIA

Hyprbinds supports the VIA family of dynamic keymap commands and VIA keyboard definition JSON as a compatibility path for non-Vial boards.

- Website: https://www.caniusevia.com/
- Specification: https://www.caniusevia.com/docs/specification

VIA and Vial are independent upstream projects. Their names are used here only to describe protocol/firmware compatibility.

## Hyprland

Hyprbinds configures and correlates keyboard behavior with Hyprland configuration and runtime semantics.

- Project: Hyprland
- Source: https://github.com/hyprwm/Hyprland
- Website: https://hypr.land/

Hyprbinds is not an official Hyprland project.

## GTK and Rust ecosystem

Hyprbinds is built with GTK4 and Rust and depends on additional crates listed in `Cargo.toml` / `Cargo.lock`. Each dependency remains subject to its own upstream license. Distribution packaging should retain dependency license notices where required.

## Attribution policy

When code is copied or substantially adapted from an upstream GPL source, preserve the upstream copyright header when present and note the source file/commit in the adapted file or commit message. Protocol behavior reimplemented from public specifications or a GPL-compatible reference implementation is credited here and in source comments where the wire semantics are non-obvious.
