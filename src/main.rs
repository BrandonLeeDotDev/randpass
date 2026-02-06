use std::env;

mod cli;
mod exits;
mod pass;
mod rand;
mod settings;
mod terminal;
mod tui;

use settings::Settings;

fn main() {
    exits::reset_terminal();
    exits::install_handlers();
    unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0) };

    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 if !Settings::has_saved_command() => tui::run(),
        _ => cli::run(args),
    }
}
