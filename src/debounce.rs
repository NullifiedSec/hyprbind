//! Debounced callbacks for realtime autosave.

use gtk4::glib::{self, ControlFlow};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub const AUTOSAVE_DELAY_MS: u64 = 800;

/// Token-based debouncer: only the latest scheduled callback runs.
#[derive(Clone)]
pub struct Debouncer {
    generation: Rc<Cell<u64>>,
}

impl Debouncer {
    pub fn new() -> Self {
        Self {
            generation: Rc::new(Cell::new(0)),
        }
    }

    /// Schedule `f` to run after `AUTOSAVE_DELAY_MS`. Cancels any prior pending run.
    pub fn schedule<F>(&self, f: F)
    where
        F: FnOnce() + 'static,
    {
        self.schedule_after(AUTOSAVE_DELAY_MS, f);
    }

    /// Schedule `f` after a custom delay in milliseconds.
    pub fn schedule_after<F>(&self, delay_ms: u64, f: F)
    where
        F: FnOnce() + 'static,
    {
        let token = self.generation.get().wrapping_add(1);
        self.generation.set(token);
        let generation = Rc::clone(&self.generation);
        let mut f = Some(f);
        glib::timeout_add_local(Duration::from_millis(delay_ms), move || {
            if generation.get() != token {
                return ControlFlow::Break;
            }
            if let Some(cb) = f.take() {
                cb();
            }
            ControlFlow::Break
        });
    }

    /// Back-compat alias used by some UI pages.
    pub fn run<F>(&self, f: F)
    where
        F: FnOnce() + 'static,
    {
        self.schedule(f);
    }

    pub fn cancel(&self) {
        let token = self.generation.get().wrapping_add(1);
        self.generation.set(token);
    }
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new()
    }
}
