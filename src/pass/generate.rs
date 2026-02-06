//! Password generation.

use std::fs::OpenOptions;
use std::io::Write;

use zeroize::Zeroize;

use super::charset;
use crate::rand::Rand;
use crate::settings::Settings;

/// Generate multiple passwords to clipboard buffer, file, or stdout.
pub fn generate_batch(settings: &Settings, count: usize) -> Option<String> {
    // Fast path: pre-build charset when not viewing seeds
    if !settings.view_chars_str {
        let mut chars = charset::build(settings);
        return generate_batch_fast(settings, count, &mut chars);
    }

    // Slow path: rebuild charset each time (for debug seed view)
    generate_batch_slow(settings, count)
}

fn generate_batch_fast(settings: &Settings, count: usize, chars: &mut [u8]) -> Option<String> {
    let mut passwords = String::new();
    let mut buf = Vec::with_capacity(settings.pass_length + 1);

    let mut file: Option<super::SecureBufWriter<std::fs::File>> = None;
    if !settings.output_file_path.is_empty() {
        file = Some(super::SecureBufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&settings.output_file_path)
                .expect("Failed to open output file"),
        ));
    }

    let stdout = std::io::stdout();
    let mut out = super::SecureBufWriter::new(stdout.lock());

    for _ in 0..count {
        generate_from_charset(chars, settings.pass_length, &mut buf);
        if settings.to_clipboard {
            // Safety: buf contains only ASCII bytes from charset
            passwords.push_str(unsafe { std::str::from_utf8_unchecked(&buf) });
            passwords.push('\n');
        } else {
            buf.push(b'\n');
            if let Some(ref mut f) = file {
                let _ = f.write_all(&buf);
            } else {
                let _ = out.write_all(&buf);
            }
        }
        buf.zeroize();
    }

    if settings.to_clipboard {
        return Some(passwords);
    }
    None
}

fn generate_batch_slow(settings: &Settings, count: usize) -> Option<String> {
    let mut passwords = String::new();

    let mut file: Option<super::SecureBufWriter<std::fs::File>> = None;
    if !settings.output_file_path.is_empty() {
        file = Some(super::SecureBufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&settings.output_file_path)
                .expect("Failed to open output file"),
        ));
    }

    let stdout = std::io::stdout();
    let mut out = super::SecureBufWriter::new(stdout.lock());

    for _ in 0..count {
        let mut pass = generate(settings);
        pass.push('\n');
        if settings.to_clipboard {
            passwords.push_str(&pass);
        } else if let Some(ref mut f) = file {
            let _ = f.write_all(pass.as_bytes());
        } else {
            let _ = out.write_all(pass.as_bytes());
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
        let rand_str = std::str::from_utf8(&chars).unwrap_or("");
        println!("Seed: ");
        println!("|- Base: {}", rand_str);
    }

    shuffle(&mut chars);

    if settings.view_chars_str {
        let rand_str = std::str::from_utf8(&chars).unwrap_or("");
        println!("|- Rand: {}", rand_str);
        if settings.output_to_terminal {
            print!("Pass:    ");
        }
    }

    let bytes: Vec<u8> = (0..settings.pass_length)
        .map(|_| random_byte(&chars, Rand::get()))
        .collect();
    // Safety: charset is all ASCII
    unsafe { String::from_utf8_unchecked(bytes) }
}

/// Fast path: generate from pre-built charset (no debug output).
/// Shuffles in place, fills buf with password bytes.
/// Caller owns the buffer — clear/zeroize between calls.
#[inline]
pub fn generate_from_charset(chars: &mut [u8], length: usize, buf: &mut Vec<u8>) {
    shuffle(chars);
    buf.clear();
    buf.extend((0..length).map(|_| random_byte(chars, Rand::get())));
}

#[inline]
fn random_byte(chars: &[u8], rng: usize) -> u8 {
    chars[rng % chars.len()]
}

#[inline]
fn shuffle(chars: &mut [u8]) {
    let rng = Rand::get();
    for i in (1..chars.len()).rev() {
        let j = rng % (i + 1);
        chars.swap(i, j);
    }
}
