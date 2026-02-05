// sort rand_pass.txt | uniq -c | sort -nr > duplicates.txt
// awk '{ count[$1]++ } END { for (num in count) print num, count[num] }' duplicates.txt

use std::env;
pub use std::fs::OpenOptions;
pub use std::io::{self, Write};
pub use std::path::Path;
use std::process::exit;

mod cli;
mod file_io;
mod menus;
mod rand;

use cli::{output_bytes, parse_byte_count};
use file_io::{load_settings_from_file, save_settings_to_file, output_passwords};
use menus::*;
use rand::*;


use copypasta::{ClipboardContext, ClipboardProvider};
use zeroize::Zeroize;

/// Reset terminal to sane state using termios directly
fn reset_terminal_termios() {
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(0, &mut termios) == 0 {
            // Enable output processing (OPOST) which includes ONLCR (NL->CR-NL)
            termios.c_oflag |= libc::OPOST | libc::ONLCR;
            // Enable canonical mode and echo
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
            libc::write(1, b"\x1b[0m\x1b[?25h\r\n".as_ptr() as *const libc::c_void, 11);
        }
    }
    if rand::is_urandom_enabled() {
        rand::disable_urandom();
    }
}

/// Signal handler - just exit, atexit handles cleanup
extern "C" fn signal_handler(_: libc::c_int) {
    unsafe { libc::exit(130) }
}

fn install_signal_handlers() {
    unsafe {
        libc::atexit(cleanup_on_exit);
        libc::signal(libc::SIGINT, signal_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, signal_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGHUP, signal_handler as *const () as libc::sighandler_t);
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pass_length: usize,
    number_of_passwords: usize,
    skip_countdown: bool,
    view_chars_str: bool,
    special_chars: Vec<char>,
    randomize_seed_chars: usize,
    special_char_density: usize,
    numeric_char_density: usize,
    lowercase_char_density: usize,
    uppercase_char_density: usize,
    output_file_path: String,
    output_to_terminal: bool,
    cli_command: String,
    to_clipboard: bool,
}

impl Settings {
    pub fn load_from_file() -> Self {
        let mut settings = Settings::default();
        let _ = load_settings_from_file(&mut settings);
        settings
    }

    pub fn save_to_file(&self) {
        let _ = save_settings_to_file(&self);
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pass_length: 74,
            number_of_passwords: 19,
            skip_countdown: false,
            view_chars_str: false,
            special_chars: vec!['!', '@', '#', '$', '%', '^', '&', '*'],
            randomize_seed_chars: 5,
            special_char_density: 1,
            numeric_char_density: 1,
            lowercase_char_density: 1,
            uppercase_char_density: 1,
            output_file_path: String::from(""),
            output_to_terminal: true,
            cli_command: String::new(),
            to_clipboard: false,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    reset_terminal_termios();
    install_signal_handlers();

    let args: Vec<String> = env::args().collect();

    // No args -> interactive mode
    if args.len() == 1 {
        gen_main_menu();
        return Ok(());
    }

    // Parse all flags in one pass
    let flags = match cli::parse(&args) {
        Ok(f) => f,
        Err(e) => {
            clear_terminal();
            println!("\x1b[31mError: {}\x1b[0m", e);
            print_help();
            exit(1);
        }
    };

    // Handle flags in priority order
    if flags.help {
        print_help();
        return Ok(());
    }

    if flags.version {
        println!("randpass {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if flags.urandom {
        if !rand::enable_urandom() {
            eprintln!("\x1b[33mWarning: /dev/urandom not available, using hardware entropy\x1b[0m");
        }
    }

    if flags.bytes {
        // Parse byte limit from -n (supports K, M, G suffixes)
        let limit = flags.number_raw.as_ref().and_then(|s| parse_byte_count(s));
        let file_path = flags.output.as_deref();
        output_bytes(limit, file_path);
        return Ok(());
    }

    // Load settings
    let mut saved_settings = Settings::load_from_file();
    let mut settings = if flags.saved {
        saved_settings.clone()
    } else {
        let mut default = Settings::default();
        default.cli_command = saved_settings.cli_command.clone();
        default
    };

    // Handle command mode
    if flags.command {
        let command = args[1..].iter()
            .filter(|a| *a != "-c" && *a != "--command")
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        saved_settings.cli_command = command.clone();
        saved_settings.save_to_file();
        settings.cli_command = command;
    }

    // Apply saved command if no explicit args given
    if !settings.cli_command.is_empty() && !flags.command && !flags.has_explicit_args() {
        // Re-parse with saved command args
        let mut combined_args = vec![args[0].clone()];
        combined_args.extend(settings.cli_command.split_whitespace().map(String::from));
        if let Ok(saved_flags) = cli::parse(&combined_args) {
            if let Some(len) = saved_flags.length {
                settings.pass_length = len;
            }
            if let Some(num) = saved_flags.number {
                settings.number_of_passwords = num;
            }
        }
    }

    // Apply explicit length/number
    if let Some(len) = flags.length {
        settings.pass_length = len;
    }
    if let Some(num) = flags.number {
        settings.number_of_passwords = num;
    }

    // Apply character set flags
    if flags.no_special {
        settings.special_char_density = 0;
    }
    if flags.hex {
        // Hex mode: only 0-9, a-f
        settings.special_char_density = 0;
        settings.uppercase_char_density = 0;
        settings.lowercase_char_density = 0;
        settings.numeric_char_density = 0;
        settings.special_chars = "0123456789abcdef".chars().collect();
        settings.special_char_density = 1;
    }
    if let Some(ref chars) = flags.special {
        settings.special_chars = chars.chars().collect();
    }

    // Apply output file
    if let Some(ref path) = flags.output {
        settings.output_file_path = if path.ends_with('/') || path == "." {
            if path == "." {
                "rand_pass.txt".to_string()
            } else {
                format!("{}rand_pass.txt", path)
            }
        } else if !path.ends_with(".txt") {
            format!("{}.txt", path)
        } else {
            path.clone()
        };
        settings.output_to_terminal = false;
    }

    // Handle clipboard
    let mut ctx = None;
    if flags.clipboard {
        match ClipboardContext::new() {
            Ok(c) => {
                ctx.replace(c);
                settings.to_clipboard = true;
            }
            Err(_) => {
                eprint!("Clipboard unavailable. Print to terminal instead? [Y/n]: ");
                let _ = io::stderr().flush();
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_ok() {
                    let input = input.trim().to_lowercase();
                    if input.is_empty() || input == "y" || input == "yes" {
                        settings.to_clipboard = false;
                        eprintln!();
                    } else {
                        eprintln!("\nAborted.");
                        return Ok(());
                    }
                } else {
                    settings.to_clipboard = false;
                }
            }
        }
    }

    // Generate passwords
    let count = flags.number.unwrap_or(1);

    if settings.to_clipboard {
        // Clipboard mode: collect passwords
        let passwords = generate_passwords_with_count(&settings, count);
        if let (Some(ctx), Some(passwords)) = (ctx.as_mut(), passwords) {
            match ctx.set_contents(passwords) {
                Ok(_) => {
                    if let Ok(mut retrieved) = ctx.get_contents() {
                        retrieved.zeroize();
                    }
                    if !flags.quiet {
                        println!("*** -COPIED TO CLIPBOARD- ***");
                    }
                }
                Err(e) => {
                    if !flags.quiet {
                        eprintln!("Clipboard error: {}", e);
                    }
                }
            }
        }
    } else if !settings.output_file_path.is_empty() && count >= 50000 && !flags.quiet {
        // Bulk file output: use TUI progress bar, skip countdown
        let mut cli_settings = settings.clone();
        cli_settings.skip_countdown = true;
        cli_settings.number_of_passwords = count;
        output_passwords(&cli_settings);
    } else if !settings.output_file_path.is_empty() {
        // File output without progress bar
        generate_passwords_with_count(&settings, count);
        if !flags.quiet {
            let full_path = std::fs::canonicalize(&settings.output_file_path)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| settings.output_file_path.clone());
            println!("{} password(s) → {}", count, full_path);
        }
    } else {
        // Terminal output
        generate_passwords_with_count(&settings, count);
    }

    Ok(())
}

fn generate_passwords_with_count(settings: &Settings, count: usize) -> Option<String> {
    let mut passwords = String::new();

    // Determine output destination
    let mut file: Option<std::fs::File> = None;
    if !settings.output_file_path.is_empty() {
        file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&settings.output_file_path)
                .expect("Failed to open output file"),
        );
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for _ in 0..count {
        let mut pass = generate_password(settings);
        if settings.to_clipboard {
            passwords.push_str(&pass);
            passwords.push('\n');
        } else if let Some(ref mut f) = file {
            let _ = f.write_all(pass.as_bytes());
            let _ = f.write_all(b"\n");
        } else {
            let _ = out.write_all(pass.as_bytes());
            let _ = out.write_all(b"\n");
        }
        pass.zeroize();
    }

    if settings.to_clipboard {
        return Some(passwords);
    }
    None
}

// -------------------------------------------------------------- PASSWORD GENERATION -------------------------------------------------------------- //

#[inline]
fn random_char(chars: &[char], rng: usize) -> char {
    chars[rng % chars.len()]
}

#[inline]
fn shuffle_chars(chars: &mut Vec<char>) {
    let rng = Rand::get() as usize;
    for i in (1..chars.len()).rev() {
        let j = rng % (i + 1);
        chars.swap(i, j);
    }
}

fn generate_password(settings: &Settings) -> String {
    let mut chars: Vec<char> = Vec::new();

    for _ in 0..settings.lowercase_char_density {
        "abcdefghijklmnopqrstuvwxyz"
            .chars()
            .for_each(|c| chars.push(c));
    }

    for _ in 0..settings.uppercase_char_density {
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
            .chars()
            .for_each(|c| chars.push(c));
    }

    for _ in 0..settings.numeric_char_density {
        chars.extend(&['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
    }

    for _ in 0..settings.special_char_density {
        chars.extend(&settings.special_chars);
    }

    if settings.view_chars_str {
        println!();
        let rand_str = chars.iter().collect::<String>();
        println!("Seed: ");
        println!("|- Base: {}", rand_str);
    }

    shuffle_chars(&mut chars);

    if settings.view_chars_str {
        let rand_str = chars.iter().collect::<String>();
        println!("|- Rand: {}", rand_str);
        if settings.output_to_terminal {
            print!("Pass:    ");
        }
    }

    (0..settings.pass_length)
        .map(|_| random_char(&chars, Rand::get() as usize))
        .collect()
}
