//! Exit handling: signal handlers, cleanup, and graceful shutdown.

use crate::rand;

/// Reset terminal to sane state using termios directly
fn reset_terminal_termios() {
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(0, &mut termios) == 0 {
            termios.c_oflag |= libc::OPOST | libc::ONLCR;
            termios.c_lflag |= libc::ICANON | libc::ECHO | libc::ISIG;
            libc::tcsetattr(0, libc::TCSANOW, &termios);
        }
    }
}

/// Cleanup function registered with atexit - runs on any exit
extern "C" fn cleanup_on_exit() {
    reset_terminal_termios();
    // Only print escape codes if stdout is a TTY (not when piping)
    unsafe {
        if libc::isatty(1) == 1 {
            libc::write(
                1,
                b"\x1b[0m\x1b[?25h\r\n".as_ptr() as *const libc::c_void,
                11,
            );
        }
    }
    if rand::is_urandom_enabled() {
        rand::disable_urandom();
    }
    // Always zeroize hardware RNG state
    rand::zeroize_state();
}

/// Signal handler for SIGINT/SIGTERM/SIGHUP - exit cleanly, atexit handles cleanup
extern "C" fn signal_handler(_: libc::c_int) {
    unsafe { libc::exit(130) }
}

/// Crash handler for SIGSEGV/SIGABRT - zero sensitive memory, then re-raise for core dump
extern "C" fn crash_handler(sig: libc::c_int) {
    unsafe {
        // Emergency zero the urandom pool (async-signal-safe)
        rand::urand::emergency_zero();
        // Zeroize hardware RNG state
        rand::zeroize_state();
        // Reset signal handler to default and re-raise for proper crash handling
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

/// Install all signal handlers and register atexit cleanup.
/// Call this early in main().
pub fn install_handlers() {
    unsafe {
        libc::atexit(cleanup_on_exit);
        libc::signal(
            libc::SIGINT,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGHUP,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGSEGV,
            crash_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGABRT,
            crash_handler as *const () as libc::sighandler_t,
        );
    }
}

/// Reset terminal state (public for use in other modules)
pub fn reset_terminal() {
    reset_terminal_termios();
}
