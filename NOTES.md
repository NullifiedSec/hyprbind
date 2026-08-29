2026-08-29 - src/lua/collect_binds.lua - corrected generic spec source stack depth - prevent saves targeting deleted collector temp files
2026-08-29 - src/extra_ui.rs - replaced raw animation speed input with a 50ms–2s duration slider - make per-animation timing obvious and prevent absurd durations
2026-08-30 - src/qol.rs - added bounded sliders and semantic dropdowns over raw settings entries with custom-value fallback - favor sane everyday controls without rewriting existing config state
2026-08-30 - src/qol.rs - made slider values clickable raw-number editors with explicit out-of-range escape hatches - keep sane defaults without blocking advanced values
