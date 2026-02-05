//! Global quiet mode state for CLI.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global quiet mode flag - suppresses warnings and prompts
static QUIET: AtomicBool = AtomicBool::new(false);

/// Enable quiet mode (suppress warnings and non-essential output)
pub fn set(quiet: bool) {
    QUIET.store(quiet, Ordering::SeqCst);
}

/// Check if quiet mode is enabled
pub fn enabled() -> bool {
    QUIET.load(Ordering::Relaxed)
}

/// Check if stdin is a tty (interactive)
pub fn is_interactive() -> bool {
    unsafe { libc::isatty(0) == 1 }
}

/// Returns true if we should skip interactive prompts.
/// True when quiet mode is enabled OR stdin is not a tty.
pub fn skip_prompt() -> bool {
    enabled() || !is_interactive()
}
