//! CLI context - bundles settings, flags, and clipboard state.

use copypasta::{ClipboardContext, ClipboardProvider};
use zeroize::Zeroize;

use super::{CliFlags, CommandMode, output_bytes, parse_byte_count, prompts, quiet};
use crate::pass;
use crate::rand;
use crate::settings::Settings;
use crate::tui::print_help;

/// Early exit - not an error, just done.
pub struct Done;

/// Application context for CLI mode.
pub struct Context {
    pub settings: Settings,
    pub saved_settings: Settings,
    pub clipboard: Option<ClipboardContext>,
    pub flags: CliFlags,
    args: Vec<String>,
}

impl Context {
    /// Create a new context by parsing command-line arguments.
    /// Returns Err with the error message if parsing fails.
    pub fn new(args: Vec<String>) -> Result<Self, String> {
        let flags = super::parse(&args).map_err(|e| e.to_string())?;

        let saved_settings = Settings::load_from_file().unwrap_or_else(|e| {
            prompts::warn(&format!("Failed to load settings: {}", e));
            Settings::default()
        });

        let settings = if flags.saved {
            saved_settings.clone()
        } else {
            Settings {
                cli_command: saved_settings.cli_command.clone(),
                number_of_passwords: 1, // CLI default, not interactive default (19)
                ..Default::default()
            }
        };

        Ok(Self {
            settings,
            saved_settings,
            clipboard: None,
            flags,
            args,
        })
    }

    /// Run CLI. Returns `Err(Done)` for early exits, `Ok(())` on completion.
    pub fn run(&mut self) -> Result<(), Done> {
        self.handle_info_flags()?;
        self.handle_command_mode()?;
        self.apply_flags();
        quiet::set(self.flags.quiet);
        self.handle_urandom();
        self.handle_bytes()?;
        self.generate_output();
        Ok(())
    }

    fn handle_info_flags(&self) -> Result<(), Done> {
        if self.flags.help {
            print_help();
            return Err(Done);
        }
        if self.flags.version {
            println!("randpass {}", env!("CARGO_PKG_VERSION"));
            return Err(Done);
        }
        Ok(())
    }

    fn handle_command_mode(&mut self) -> Result<(), Done> {
        match self.flags.command {
            CommandMode::Get => {
                if self.settings.cli_command.is_empty() {
                    println!("(no saved command)");
                } else {
                    println!("{}", self.settings.cli_command);
                }
                Err(Done)
            }
            CommandMode::Unset => {
                self.saved_settings.cli_command.clear();
                if let Err(e) = self.saved_settings.save_to_file() {
                    prompts::warn(&format!("Failed to clear command: {}", e));
                }
                Err(Done)
            }
            CommandMode::Set | CommandMode::None => Ok(()),
        }
    }

    fn handle_urandom(&self) {
        if self.flags.urandom && !rand::enable_urandom() {
            prompts::urandom_unavailable();
        }
    }

    fn handle_bytes(&self) -> Result<(), Done> {
        if self.flags.bytes {
            let limit = self
                .flags
                .number_raw
                .as_ref()
                .and_then(|s| parse_byte_count(s));
            output_bytes(limit, self.flags.output.as_deref());
            return Err(Done);
        }
        Ok(())
    }

    /// Apply CLI flags to settings.
    fn apply_flags(&mut self) {
        // Handle command set mode
        if self.flags.command == CommandMode::Set {
            let command = self.args[1..]
                .iter()
                .filter(|a| *a != "-c" && *a != "--command" && *a != "set")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            self.saved_settings.cli_command = command.clone();
            if let Err(e) = self.saved_settings.save_to_file() {
                prompts::warn(&format!("Failed to save command: {}", e));
            }
            self.settings.cli_command = command;
        }

        // Apply saved command if no explicit args given
        if !self.settings.cli_command.is_empty()
            && self.flags.command == CommandMode::None
            && !self.flags.has_explicit_args()
        {
            let mut combined_args = vec![self.args[0].clone()];
            combined_args.extend(
                self.settings
                    .cli_command
                    .split_whitespace()
                    .map(String::from),
            );
            if let Ok(saved_flags) = super::parse(&combined_args) {
                // Replace flags with saved flags so all flag handling applies
                self.flags = saved_flags;
            }
        }

        // Apply explicit length/number
        if let Some(len) = self.flags.length {
            self.settings.pass_length = len;
        }
        if let Some(num) = self.flags.number {
            self.settings.number_of_passwords = num;
        }

        // Apply character set flags
        if self.flags.no_special {
            self.settings.special_char_density = 0;
        }
        if self.flags.hex {
            self.settings.special_char_density = 0;
            self.settings.uppercase_char_density = 0;
            self.settings.lowercase_char_density = 0;
            self.settings.numeric_char_density = 0;
            self.settings.special_chars = b"0123456789abcdef".to_vec();
            self.settings.special_char_density = 1;
        }
        if let Some(ref chars) = self.flags.special {
            self.settings.special_chars = chars.bytes().collect();
        }

        // Apply output file
        if let Some(ref path) = self.flags.output {
            self.settings.output_file_path = if path.ends_with('/') || path == "." {
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
            self.settings.output_to_terminal = false;
        }

        // Handle clipboard
        if self.flags.clipboard {
            match ClipboardContext::new() {
                Ok(c) => {
                    self.clipboard = Some(c);
                    self.settings.to_clipboard = true;
                }
                Err(_) => {
                    if prompts::clipboard_fallback_prompt() {
                        self.settings.to_clipboard = false;
                    } else {
                        std::process::exit(0);
                    }
                }
            }
        }
    }

    /// Generate passwords and handle output.
    pub fn generate_output(&mut self) {
        // Use explicit flag, else settings (which may come from saved command)
        let count = self
            .flags
            .number
            .unwrap_or(self.settings.number_of_passwords.max(1));

        if self.settings.to_clipboard {
            let passwords = pass::generate_batch(&self.settings, count);
            if let (Some(ctx), Some(mut passwords)) = (self.clipboard.as_mut(), passwords) {
                match ctx.set_contents(passwords.clone()) {
                    Ok(_) => {
                        if let Ok(mut retrieved) = ctx.get_contents() {
                            retrieved.zeroize();
                        }
                        prompts::clipboard_copied();
                    }
                    Err(e) => {
                        prompts::clipboard_error(&e.to_string());
                    }
                }
                passwords.zeroize();
            }
        } else if !self.settings.output_file_path.is_empty()
            && count >= 500_000
            && !self.flags.quiet
        {
            // Bulk file output: use TUI progress bar
            let mut cli_settings = self.settings.clone();
            cli_settings.skip_countdown = true;
            cli_settings.number_of_passwords = count;
            pass::output::with_progress(&cli_settings);
        } else if !self.settings.output_file_path.is_empty() {
            // File output without progress bar
            pass::generate_batch(&self.settings, count);
            let full_path = std::fs::canonicalize(&self.settings.output_file_path)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| self.settings.output_file_path.clone());
            prompts::passwords_written(count, &full_path);
        } else {
            // Terminal output
            pass::generate_batch(&self.settings, count);
        }
    }
}
