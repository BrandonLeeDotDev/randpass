//! Password generation.

use std::fs::OpenOptions;
use std::io::Write;

use zeroize::Zeroize;

use super::charset;
use crate::rand::Rand;
use crate::settings::Settings;

/// Generate multiple passwords to clipboard buffer, file, or stdout.
pub fn generate_batch(settings: &Settings, count: usize) -> Option<String> {
    let mut passwords = String::new();

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
        let mut pass = generate(settings);
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

/// Generate a single password based on settings.
pub fn generate(settings: &Settings) -> String {
    let mut chars = charset::build(settings);

    if settings.view_chars_str {
        println!();
        let rand_str = chars.iter().collect::<String>();
        println!("Seed: ");
        println!("|- Base: {}", rand_str);
    }

    shuffle(&mut chars);

    if settings.view_chars_str {
        let rand_str = chars.iter().collect::<String>();
        println!("|- Rand: {}", rand_str);
        if settings.output_to_terminal {
            print!("Pass:    ");
        }
    }

    (0..settings.pass_length)
        .map(|_| random_char(&chars, Rand::get()))
        .collect()
}

#[inline]
fn random_char(chars: &[char], rng: usize) -> char {
    chars[rng % chars.len()]
}

#[inline]
fn shuffle(chars: &mut [char]) {
    let rng = Rand::get();
    for i in (1..chars.len()).rev() {
        let j = rng % (i + 1);
        chars.swap(i, j);
    }
}
