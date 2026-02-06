//! Character set building for password generation.

use crate::settings::Settings;

const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";

/// Build the character pool based on density settings.
pub fn build(settings: &Settings) -> Vec<u8> {
    let mut chars: Vec<u8> = Vec::new();

    for _ in 0..settings.lowercase_char_density {
        chars.extend_from_slice(LOWERCASE);
    }

    for _ in 0..settings.uppercase_char_density {
        chars.extend_from_slice(UPPERCASE);
    }

    for _ in 0..settings.numeric_char_density {
        chars.extend_from_slice(DIGITS);
    }

    for _ in 0..settings.special_char_density {
        chars.extend_from_slice(&settings.special_chars);
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
