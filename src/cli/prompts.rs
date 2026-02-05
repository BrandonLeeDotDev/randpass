//! Centralized warning and prompt messages for CLI output.

use std::io::Write;

use super::quiet;

// ANSI color codes
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

/// Print a warning message to stderr (yellow) - suppressed in quiet mode
pub fn warn(msg: &str) {
    if !quiet::enabled() {
        eprintln!("{YELLOW}{msg}{RESET}");
    }
}

/// Print an error message to stderr (red) - NOT suppressed (errors are always shown)
pub fn error(msg: &str) {
    eprintln!("{RED}{msg}{RESET}");
}

/// Print mlock failure warning with fix instructions
pub fn mlock_failed() {
    warn("Warning: mlock failed - entropy pool may be swapped to disk.");
    warn("Fix: ulimit -l unlimited, or setcap cap_ipc_lock=ep on binary");
}

/// Prompt user to continue after mlock failure. Returns true if user agrees.
/// Only prompts if stdin is a tty, otherwise returns true (continue).
/// In quiet mode, silently continues.
pub fn mlock_continue_prompt() -> bool {
    if quiet::skip_prompt() {
        return true; // Non-interactive or quiet: continue silently
    }

    eprint!("{YELLOW}Continue anyway? [y/N]: {RESET}");
    let _ = std::io::stderr().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        if input == "y" || input == "yes" {
            return true;
        }
    }

    eprintln!("Aborted. Using hardware RNG instead.");
    false
}

/// Print urandom unavailable warning
pub fn urandom_unavailable() {
    warn("Warning: /dev/urandom not available, using hardware entropy");
}

/// Print clipboard copied confirmation - suppressed in quiet mode
pub fn clipboard_copied() {
    if !quiet::enabled() {
        println!("*** -COPIED TO CLIPBOARD- ***");
    }
}

/// Print clipboard error - NOT suppressed (errors are always shown)
pub fn clipboard_error(err: &str) {
    eprintln!("Clipboard error: {err}");
}

/// Prompt user when clipboard is unavailable. Returns true to fallback to terminal, false to abort.
/// In quiet/non-interactive mode, silently falls back to terminal.
pub fn clipboard_fallback_prompt() -> bool {
    if quiet::skip_prompt() {
        return true; // Fallback silently
    }

    eprint!("Clipboard unavailable. Print to terminal instead? [Y/n]: ");
    let _ = std::io::stderr().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        if input.is_empty() || input == "y" || input == "yes" {
            eprintln!();
            return true;
        }
    } else {
        return true; // Fallback on read error
    }

    eprintln!("\nAborted.");
    false
}

/// Print password output summary - suppressed in quiet mode
pub fn passwords_written(count: usize, path: &str) {
    if !quiet::enabled() {
        println!("{count} password(s) \u{2192} {path}");
    }
}
