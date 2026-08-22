# Third-Party Notices and Credits

Hyprbinds is GPL-2.0 software. This project interoperates with and, where noted, may reuse or adapt behavior from the following open-source projects.

## Vial GUI

- Project: `vial-kb/vial-gui`
- Website: https://get.vial.today/
- Source: https://github.com/vial-kb/vial-gui
- License: GNU General Public License v2.0
- Pinned compatibility reference used by this branch: `aef8222a2d0429a183b2ed692d5f9efcfd383f08`

Vial is the canonical upstream compatibility target for Vial firmware configuration. Hyprbinds' native Rust implementation is intended to interoperate with Vial-capable keyboards and to reach feature parity while adding Hyprland-aware keybind correlation.

When `scripts/sync-vial-upstream.sh` is used, an unmodified checkout of the official Vial GUI is placed under `third_party/vial-gui/`. Its original copyright notices and `COPYING` file remain authoritative for that checkout.

## QMK Firmware

Vial firmware is built on QMK and the dynamic-keymap/keycode concepts exposed here come from the QMK ecosystem.

- Project: QMK Firmware
- Source: https://github.com/qmk/qmk_firmware
- Website: https://qmk.fm/

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

When code is copied or substantially adapted from an upstream GPL source, preserve the upstream copyright header when present and note the source file/commit in the adapted file or commit message. Protocol behavior reimplemented independently from public specifications should still credit the protocol/project in this notice when appropriate.
