//! Terminal output utilities.
//!
//! Box drawing, progress bars, number formatting, ANSI helpers.

use crossterm::terminal::disable_raw_mode;
use std::io::{self, Write};

// ============================================================================
// ANSI Color/Style Constants
// ============================================================================

pub const RESET: &str = "\x1b[0m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const RED: &str = "\x1b[38;5;9m";

// ============================================================================
// Terminal Control
// ============================================================================

/// Clear screen and move cursor to top-left.
pub fn clear() {
    print!("\x1b[2J\x1b[3J\x1b[H");
    flush();
}

/// Flush stdout.
pub fn flush() {
    let _ = io::stdout().flush();
}

/// Reset terminal to sane state (fixes staggered text issues).
pub fn reset_terminal() {
    let _ = disable_raw_mode();
    print!("\x1b[0m");
    flush();
}

// ============================================================================
// Styled Output Helpers
// ============================================================================

/// Print error message in red.
pub fn print_error(msg: &str) {
    println!("{RED}{msg}{RESET}");
}

/// Print a horizontal rule (box style).
pub fn print_rule() {
    println!("├{}┤", "─".repeat(BOX_WIDTH - 2));
}

// ============================================================================
// Number Formatting
// ============================================================================

pub fn format_number(num: usize) -> String {
    let s = num.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
}

// ============================================================================
// Box Drawing (74 char width)
// ============================================================================

pub const BOX_WIDTH: usize = 74;

/// Print box top with optional title: ┌─ Title ───────────────────────────┐
pub fn box_top(title: &str) {
    if title.is_empty() {
        println!("┌{}┐", "─".repeat(BOX_WIDTH - 2));
    } else {
        let title_part = format!("─ {} ", title);
        let remaining = BOX_WIDTH - 2 - title_part.chars().count();
        println!("┌{}{}┐", title_part, "─".repeat(remaining));
    }
}

/// Print box content line: │ content                                        │
pub fn box_line(content: &str) {
    let inner_width = BOX_WIDTH - 4;
    let display_len = console_width(content);

    if display_len <= inner_width {
        let padding = inner_width - display_len;
        println!("│ {}{} │", content, " ".repeat(padding));
    } else {
        println!("│ {} │", content);
    }
}

/// Print centered box content line: │          content          │
pub fn box_line_center(content: &str) {
    let inner_width = BOX_WIDTH - 4;
    let display_len = console_width(content);

    if display_len <= inner_width {
        let total_padding = inner_width - display_len;
        let left_pad = total_padding / 2;
        let right_pad = total_padding - left_pad;
        println!(
            "│ {}{}{} │",
            " ".repeat(left_pad),
            content,
            " ".repeat(right_pad)
        );
    } else {
        println!("│ {} │", content);
    }
}

/// Print box bottom: └───────────────────────────────────────────────────────┘
pub fn box_bottom() {
    println!("└{}┘", "─".repeat(BOX_WIDTH - 2));
}

/// Print a help option with flag and description, auto-wrapping if needed.
pub fn box_opt(flag: &str, desc: &str) {
    let inner_width = BOX_WIDTH - 4;
    let flag_col = 27;
    let desc_col = inner_width - flag_col;

    let flag_padded = if flag.len() < flag_col {
        format!("{}{}", flag, " ".repeat(flag_col - flag.len()))
    } else {
        flag[..flag_col].to_string()
    };

    let words: Vec<&str> = desc.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= desc_col {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if let Some(first) = lines.first() {
        let padding = desc_col.saturating_sub(first.len());
        println!("│ {}{}{} │", flag_padded, first, " ".repeat(padding));
    } else {
        let padding = desc_col;
        println!("│ {}{} │", flag_padded, " ".repeat(padding));
    }

    let indent = " ".repeat(flag_col);
    for line in lines.iter().skip(1) {
        let padding = desc_col.saturating_sub(line.len());
        println!("│ {}{}{} │", indent, line, " ".repeat(padding));
    }
}

/// Calculate display width accounting for ANSI escape codes.
fn console_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            width += 1;
        }
    }
    width
}

/// Print centered text within box width.
pub fn print_centered(text: &str) {
    let padding = BOX_WIDTH.saturating_sub(text.len()) / 2;
    print!(
        "{}{}{}\r\n",
        " ".repeat(padding),
        text,
        " ".repeat(BOX_WIDTH - padding - text.len())
    );
    let _ = std::io::stdout().flush();
}

// ============================================================================
// Progress Bar
// ============================================================================

/// Render a progress bar inside a box with centered text (3 lines).
pub fn progress_bar_box(percent: f32, stats: &str) {
    let inner_width = BOX_WIDTH - 2;
    let filled = if percent >= 100.0 {
        inner_width
    } else {
        ((percent / 100.0) * inner_width as f32) as usize
    };

    let text_chars: Vec<char> = stats.chars().collect();
    let text_len = text_chars.len();
    let padding = if text_len < inner_width {
        (inner_width - text_len) / 2
    } else {
        0
    };

    let mut content: Vec<char> = vec![' '; inner_width];
    for (i, ch) in text_chars.iter().enumerate() {
        if padding + i < inner_width {
            content[padding + i] = *ch;
        }
    }

    // Top border
    if filled > 0 {
        print!("\r▗");
        print!("{}", "▄".repeat(filled));
    } else {
        print!("\r┌");
    }
    if filled < inner_width {
        print!("{}", "─".repeat(inner_width - filled));
        print!("┐\r\n");
    } else {
        print!("▖\r\n");
    }

    // Middle
    if filled > 0 {
        print!("\r▐");
        let filled_str: String = content[..filled].iter().collect();
        print!("\x1b[7m{}\x1b[0m", filled_str);
    } else {
        print!("\r│");
    }
    if filled < inner_width {
        let unfilled_str: String = content[filled..].iter().collect();
        print!("{}", unfilled_str);
        print!("│\r\n");
    } else {
        print!("▌\r\n");
    }

    // Bottom border
    if filled > 0 {
        print!("\r▝");
        print!("{}", "▀".repeat(filled));
    } else {
        print!("\r└");
    }
    if filled < inner_width {
        print!("{}", "─".repeat(inner_width - filled));
        print!("┘\r\n");
    } else {
        print!("▘\r\n");
    }

    let _ = std::io::stdout().flush();
}

/// Render a countdown bar with bouncing grey spot and centered text (3 lines).
pub fn countdown_bar(spot_pos: usize, text: &str) {
    let inner_width = BOX_WIDTH - 2;

    let text_chars: Vec<char> = text.chars().collect();
    let text_len = text_chars.len();
    let padding = if text_len < inner_width {
        (inner_width - text_len) / 2
    } else {
        0
    };

    let mut content: Vec<char> = vec![' '; inner_width];
    for (i, ch) in text_chars.iter().enumerate() {
        if padding + i < inner_width {
            content[padding + i] = *ch;
        }
    }

    let spot = spot_pos % inner_width;

    print!("\r┌{}┐\r\n", "─".repeat(inner_width));

    print!("\r│");
    for (i, ch) in content.iter().enumerate() {
        if i == spot {
            print!("\x1b[90m█\x1b[0m");
        } else {
            print!("{}", ch);
        }
    }
    print!("│\r\n");

    print!("\r└{}┘\r\n", "─".repeat(inner_width));

    let _ = std::io::stdout().flush();
}

// ============================================================================
// Entropy Calculation
// ============================================================================

/// Calculate password entropy in bits.
pub fn calculate_entropy(password_length: usize, charset_size: usize) -> f64 {
    if charset_size == 0 {
        return 0.0;
    }
    password_length as f64 * (charset_size as f64).log2()
}

/// Get entropy strength description.
pub fn entropy_strength(bits: f64) -> &'static str {
    match bits as u32 {
        0..=35 => "Weak",
        36..=59 => "Fair",
        60..=127 => "Strong",
        _ => "Very Strong",
    }
}

/// Get info about the entropy source.
pub fn entropy_source_info() -> &'static str {
    if crate::rand::is_urandom_enabled() {
        return "/dev/urandom (32MB pool) - High quality";
    }

    #[cfg(target_arch = "x86_64")]
    {
        "rdtsc (CPU timestamp counter) - High quality"
    }

    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    {
        "pmccntr (ARM cycle counter) - High quality"
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "arm", target_arch = "aarch64")))]
    {
        "/dev/urandom (32MB pool) - High quality"
    }
}
