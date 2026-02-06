//! Password generation settings.

mod file;

#[derive(Debug, Clone)]
pub struct Settings {
    pub pass_length: usize,
    pub number_of_passwords: usize,
    pub skip_countdown: bool,
    pub view_chars_str: bool,
    pub special_chars: Vec<u8>,
    pub randomize_seed_chars: usize,
    pub special_char_density: usize,
    pub numeric_char_density: usize,
    pub lowercase_char_density: usize,
    pub uppercase_char_density: usize,
    pub output_file_path: String,
    pub output_to_terminal: bool,
    pub cli_command: String,
    pub to_clipboard: bool,
}

impl Settings {
    pub fn load_from_file() -> Result<Self, std::io::Error> {
        let mut settings = Settings::default();
        file::load(&mut settings)?;
        Ok(settings)
    }

    pub fn save_to_file(&self) -> Result<(), std::io::Error> {
        file::save(self)
    }

    pub fn has_saved_command() -> bool {
        Self::load_from_file()
            .map(|s| !s.cli_command.is_empty())
            .unwrap_or(false)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pass_length: 74,
            number_of_passwords: 19,
            skip_countdown: false,
            view_chars_str: false,
            special_chars: vec![b'!', b'@', b'#', b'$', b'%', b'^', b'&', b'*'],
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
