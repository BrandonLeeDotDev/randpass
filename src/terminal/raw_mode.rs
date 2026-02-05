//! Raw mode RAII guard.

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io;

/// Guard that ensures raw mode is disabled when dropped.
pub struct RawModeGuard {
    was_enabled: bool,
}

impl RawModeGuard {
    /// Enable raw mode, returning a guard that will disable it on drop.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self { was_enabled: true })
    }

    /// Manually disable raw mode (also happens on drop).
    pub fn disable(&mut self) {
        if self.was_enabled {
            let _ = disable_raw_mode();
            self.was_enabled = false;
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        self.disable();
    }
}
