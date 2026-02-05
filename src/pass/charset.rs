//! Character set building for password generation.

use crate::settings::Settings;

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// Build the character pool based on density settings.
pub fn build(settings: &Settings) -> Vec<char> {
    let mut chars: Vec<char> = Vec::new();

    for _ in 0..settings.lowercase_char_density {
        chars.extend(LOWERCASE.chars());
    }

    for _ in 0..settings.uppercase_char_density {
        chars.extend(UPPERCASE.chars());
    }

    for _ in 0..settings.numeric_char_density {
        chars.extend(DIGITS);
    }

    for _ in 0..settings.special_char_density {
        chars.extend(&settings.special_chars);
    }

    chars
}

/// Calculate the effective charset size (for entropy calculation).
pub fn size(settings: &Settings) -> usize {
    let mut size = 0;
    size += 26 * settings.lowercase_char_density;
    size += 26 * settings.uppercase_char_density;
    size += 10 * settings.numeric_char_density;
    size += settings.special_chars.len() * settings.special_char_density;
    size
}
