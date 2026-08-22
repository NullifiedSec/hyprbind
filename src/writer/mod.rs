//! Disk-write layer for the Hyprland config.
//!
//! Every save/add/delete goes through [`write_config_atomic`] which snapshots
//! the file via `crate::backup` and renames a temp file into place.
//!
//! Layout: most edits land inside `-- >>> hyprbinds:managed-<section>` markers
//! so hand-written Lua above the section stays untouched. See `section.rs` for
//! the primitives that insert into those markers.
//!
//! Split into:
//! - [`error`] — `WriteError`, `WriteResult`, `WriteMode`
//! - [`section`] — managed-section helpers + marker constants
//! - [`locate`] — call/span finding
//! - [`lua_text`] — Lua text manipulation utilities
//! - [`binds`] / [`variables`] / [`env`] / [`startup`] / [`submaps`] / [`rules`] —
//!   per-feature CRUD that build on the above
//! - [`spec_calls`] — generic CRUD used by monitor/device/gesture/animation/curve
//! - [`settings`] — managed config-override block
//! - [`apply`] — bulk rewrite from a `BindCollection`

mod apply;
mod binds;
mod env;
mod error;
mod locate;
mod lua_text;
mod rules;
mod section;
mod settings;
mod spec_calls;
mod startup;
mod submaps;
mod variables;

pub use apply::apply_collection;
pub use binds::{add_bind, delete_bind, save_bind};
pub use env::{add_env, delete_env, save_env};
pub use error::{WriteError, WriteMode, WriteResult};
pub use rules::{
    add_layer_rule, add_window_rule, delete_layer_rule, delete_window_rule, save_layer_rule,
    save_window_rule,
};
pub use settings::save_config_override;
pub use spec_calls::{
    add_animation, add_curve, add_device, add_gesture, add_monitor, add_workspace_rule,
    delete_animation, delete_curve, delete_device, delete_gesture, delete_monitor,
    delete_workspace_rule, save_animation, save_curve, save_device, save_gesture, save_monitor,
    save_workspace_rule, scale_animation_speeds,
};
pub use startup::{add_startup, delete_startup, save_startup};
pub use submaps::{add_submap, delete_submap};
pub use variables::{add_variable, delete_variable, save_variable};
