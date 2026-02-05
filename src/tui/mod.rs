//! Interactive TUI menus.

mod input;
mod options;
mod text;

pub use input::*;
pub use options::*;
pub use text::*;

/// Run TUI interactive mode.
pub fn run() {
    gen_main_menu();
}
