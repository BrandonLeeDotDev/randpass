//! CLI argument parsing and execution.

mod bytes;
mod context;
mod flags;
mod parse;
pub mod prompts;
pub mod quiet;

use crate::terminal::clear;
use crate::tui::print_help;
use context::Context;

pub use bytes::output as output_bytes;
pub use bytes::parse_byte_count;
pub use flags::{CliFlags, CommandMode};
pub use parse::parse;

/// Run CLI mode with given arguments.
pub fn run(args: Vec<String>) {
    let mut ctx = match Context::new(args) {
        Ok(c) => c,
        Err(e) => {
            clear();
            prompts::error(&format!("Error: {}", e));
            print_help();
            std::process::exit(1);
        }
    };
    let _ = ctx.run();
}
