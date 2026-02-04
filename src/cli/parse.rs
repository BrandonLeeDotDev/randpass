use super::CliFlags;

#[derive(Debug)]
pub enum ParseError {
    InvalidNumber(String),
    UnknownArg(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            ParseError::UnknownArg(s) => write!(f, "Unknown argument: {}", s),
        }
    }
}

pub fn parse(args: &[String]) -> Result<CliFlags, ParseError> {
    let mut flags = CliFlags::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => flags.help = true,
            "-v" | "--version" => flags.version = true,
            "-q" | "--quiet" => flags.quiet = true,
            "--bytes" => flags.bytes = true,
            "-u" | "--urandom" => flags.urandom = true,
            "-b" | "--board" => flags.clipboard = true,
            "-s" | "--saved" => flags.saved = true,
            "-d" | "--default" => flags.default = true,
            "-c" | "--command" => flags.command = true,
            "--no-special" => flags.no_special = true,
            "--hex" => flags.hex = true,
            "-l" | "--length" => {
                i += 1;
                if i < args.len() {
                    flags.length = Some(args[i].parse().map_err(|_| {
                        ParseError::InvalidNumber(args[i].clone())
                    })?);
                }
            }
            "-n" | "--number" => {
                i += 1;
                if i < args.len() {
                    flags.number_raw = Some(args[i].clone());
                    // Try parsing as plain number (for password count)
                    // If it has K/M/G suffix, this will fail but that's ok for --bytes mode
                    flags.number = args[i].parse().ok();
                }
            }
            "--special" => {
                i += 1;
                if i < args.len() {
                    flags.special = Some(args[i].clone());
                }
            }
            "-o" | "--output" => {
                // Check if next arg exists and isn't another flag
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    i += 1;
                    flags.output = Some(args[i].clone());
                } else {
                    // No path given, default to current dir
                    flags.output = Some(".".to_string());
                }
            }
            arg => return Err(ParseError::UnknownArg(arg.to_string())),
        }
        i += 1;
    }

    Ok(flags)
}
