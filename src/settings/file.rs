//! Settings file persistence.

use std::env;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::Settings;

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(get_path())?;

    let special_chars_str = settings
        .special_chars
        .iter()
        .map(|&c| match c {
            ',' => "|,".to_string(),
            '|' => "||".to_string(),
            _ => c.to_string(),
        })
        .collect::<Vec<String>>()
        .join("");

    let data = format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        settings.pass_length,
        settings.number_of_passwords,
        settings.skip_countdown,
        settings.view_chars_str,
        special_chars_str,
        settings.randomize_seed_chars,
        settings.special_char_density,
        settings.numeric_char_density,
        settings.lowercase_char_density,
        settings.uppercase_char_density,
        settings.output_file_path,
        settings.output_to_terminal,
        settings.cli_command
    );

    file.write_all(data.as_bytes())?;
    Ok(())
}

pub fn load(settings: &mut Settings) -> std::io::Result<()> {
    let path = get_path();
    if !Path::new(&path).exists()
        && let Some(parent) = Path::new(&path).parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("Failed to create directory for settings file: {}", e);
        return Ok(());
    }

    let file = OpenOptions::new()
        .read(true)
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    if line.is_empty() {
        save(settings)?;
    } else {
        let parts = split_escaped(line.trim(), ',');

        if parts.len() == 13 {
            settings.pass_length = parts[0].parse().unwrap_or(settings.pass_length);
            settings.number_of_passwords = parts[1].parse().unwrap_or(settings.number_of_passwords);
            settings.skip_countdown = parts[2].parse().unwrap_or(settings.skip_countdown);
            settings.view_chars_str = parts[3].parse().unwrap_or(settings.view_chars_str);
            settings.special_chars = parts[4].chars().collect();
            settings.randomize_seed_chars =
                parts[5].parse().unwrap_or(settings.randomize_seed_chars);
            settings.special_char_density =
                parts[6].parse().unwrap_or(settings.special_char_density);
            settings.numeric_char_density =
                parts[7].parse().unwrap_or(settings.numeric_char_density);
            settings.lowercase_char_density =
                parts[8].parse().unwrap_or(settings.lowercase_char_density);
            settings.uppercase_char_density =
                parts[9].parse().unwrap_or(settings.uppercase_char_density);
            settings.output_file_path = parts[10].to_string();
            settings.output_to_terminal = parts[11].parse().unwrap_or(settings.output_to_terminal);
            settings.cli_command = parts[12].parse().unwrap_or(settings.cli_command.clone());
        } else {
            save(settings)?;
            load(settings)?;
        }
    }

    Ok(())
}

#[inline]
fn get_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{}/.config/randpass/settings", home)
}

fn split_escaped(s: &str, delimiter: char) -> Vec<String> {
    let mut parts = vec![];
    let mut current = String::new();
    let mut escape_next = false;

    for c in s.chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
        } else if c == '|' {
            escape_next = true;
        } else if c == delimiter {
            if current.is_empty() && !parts.is_empty() {
                parts.push(String::new());
            } else {
                parts.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() || (s.ends_with(delimiter) && !escape_next) {
        parts.push(current);
    }

    parts
}
