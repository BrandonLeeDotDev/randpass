use crossterm::event::{Event, KeyCode, KeyModifiers, read};

use crate::terminal::{RawModeGuard, flush, format_number, reset_terminal};

/// Format a string of digits with comma separators
fn format_digits(s: &str) -> String {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return String::new();
    }
    if let Ok(n) = digits.parse::<usize>() {
        format_number(n)
    } else {
        digits
    }
}

/// Get numeric input with live comma formatting
pub fn get_numeric_input(prompt: &str, initial_value: usize) -> Option<usize> {
    let mut digits = if initial_value > 0 {
        initial_value.to_string()
    } else {
        String::new()
    };
    let mut cancelled = false;

    let _guard = match RawModeGuard::new() {
        Ok(g) => g,
        Err(_) => return Some(initial_value),
    };

    let formatted = format_digits(&digits);
    print!("{}: {}", prompt, formatted);
    flush();

    let mut last_display_len = formatted.len();

    loop {
        match read() {
            Ok(Event::Key(key_event)) => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        reset_terminal();
                        println!();
                        std::process::exit(0);
                    }
                    KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        cancelled = true;
                        break;
                    }
                    KeyCode::Esc => {
                        cancelled = true;
                        break;
                    }
                    KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        digits.clear();
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    KeyCode::Backspace => {
                        digits.pop();
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        digits.push(c);
                    }
                    _ => {}
                }

                // Redraw with formatting
                let formatted = format_digits(&digits);
                print!("\r{}: {}", prompt, " ".repeat(last_display_len + 1));
                print!("\r{}: {}", prompt, formatted);
                flush();
                last_display_len = formatted.len();
            }
            Err(_) => break,
            _ => {}
        }
    }

    drop(_guard);
    println!();

    if cancelled {
        None
    } else if digits.is_empty() {
        Some(0)
    } else {
        digits.parse().ok()
    }
}

pub fn get_editable_input(prompt: &str, initial_value: &str) -> Option<String> {
    let mut input = initial_value.to_string();
    let mut cursor_pos = input.len() + 1;
    let mut input_len = cursor_pos;
    let mut cancelled = false;

    // RawModeGuard ensures raw mode is disabled even if we panic or return early
    let _guard = match RawModeGuard::new() {
        Ok(g) => g,
        Err(_) => return Some(input), // Can't enable raw mode, return default
    };

    print!("{}: {}", prompt, input);
    flush();

    loop {
        match read() {
            Ok(Event::Key(key_event)) => {
                match key_event.code {
                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Reset terminal BEFORE exit since process::exit doesn't run destructors
                        reset_terminal();
                        println!();
                        std::process::exit(0);
                    }
                    KeyCode::Char('q') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        cancelled = true;
                        break;
                    }
                    KeyCode::Esc => {
                        cancelled = true;
                        break;
                    }
                    KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        input.clear();
                        cursor_pos = 1;
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    KeyCode::Backspace => {
                        if cursor_pos > 1 {
                            cursor_pos -= 1;
                            input_len -= 1;
                            input.remove(cursor_pos - 1);
                        }
                    }
                    KeyCode::Delete => {
                        if cursor_pos < input.len() + 1 {
                            input.remove(cursor_pos - 1);
                        }
                    }
                    KeyCode::Left => {
                        if cursor_pos > 1 {
                            cursor_pos -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if cursor_pos < input.len() + 1 {
                            cursor_pos += 1;
                        }
                    }
                    KeyCode::Char(c) => {
                        input.insert(cursor_pos - 1, c);
                        cursor_pos += 1;
                        input_len += 1;
                    }
                    _ => {}
                }

                // Redraw the input line
                print!("\r{}: {}", prompt, " ".repeat(input_len + 1));
                print!("\r{}: {}", prompt, input);
                flush();

                // Move cursor to correct position
                print!("\x1b[{}G", prompt.len() + 2 + cursor_pos);
                flush();
            }
            Err(_) => {
                break;
            }
            _ => {}
        }
    }

    // Explicitly drop guard to disable raw mode BEFORE println
    drop(_guard);
    println!();
    if cancelled { None } else { Some(input) }
}
