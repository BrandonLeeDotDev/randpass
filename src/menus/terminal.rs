//! Terminal utilities with proper state management
//!
//! Handles ANSI codes, raw mode cleanup, and consistent output

use std::io::{self, Write};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

// ============================================================================
// ANSI Color/Style Constants
// ============================================================================

pub const RESET: &str = "\x1b[0m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const RED: &str = "\x1b[38;5;9m";

// ============================================================================
// Terminal Control
// ============================================================================

/// Clear screen and move cursor to top-left
pub fn clear() {
    print!("\x1b[2J\x1b[3J\x1b[H");
    flush();
}

/// Flush stdout
pub fn flush() {
    let _ = io::stdout().flush();
}

/// Reset terminal to sane state (fixes staggered text issues)
pub fn reset_terminal() {
    let _ = disable_raw_mode();
    print!("\x1b[0m");
    flush();
}

// ============================================================================
// Raw Mode Guard (RAII pattern)
// ============================================================================

/// Guard that ensures raw mode is disabled when dropped
pub struct RawModeGuard {
    was_enabled: bool,
}

impl RawModeGuard {
    /// Enable raw mode, returning a guard that will disable it on drop
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self { was_enabled: true })
    }

    /// Manually disable raw mode (also happens on drop)
    pub fn disable(&mut self) {
        if self.was_enabled {
            let _ = disable_raw_mode();
            self.was_enabled = false;
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        self.disable();
    }
}

// ============================================================================
// Styled Output Helpers
// ============================================================================

/// Print error message in red
pub fn print_error(msg: &str) {
    println!("{RED}{msg}{RESET}");
}

/// Print a horizontal rule (box style)
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
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

// ============================================================================
// Box Drawing (73 char width to match menus)
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
    // Fixed inner width: 73 - 2 (borders) - 2 (padding) = 69
    let inner_width = BOX_WIDTH - 4;
    let display_len = console_width(content);

    if display_len <= inner_width {
        let padding = inner_width - display_len;
        println!("│ {}{} │", content, " ".repeat(padding));
    } else {
        // Content too long - just print it (will overflow)
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
        println!("│ {}{}{} │", " ".repeat(left_pad), content, " ".repeat(right_pad));
    } else {
        println!("│ {} │", content);
    }
}

/// Print box bottom: └───────────────────────────────────────────────────────┘
pub fn box_bottom() {
    println!("└{}┘", "─".repeat(BOX_WIDTH - 2));
}

/// Print a help option with flag and description, auto-wrapping if needed
/// flag_col_width: width of the flag column (including leading spaces)
pub fn box_opt(flag: &str, desc: &str) {
    let inner_width = BOX_WIDTH - 4; // 70
    let flag_col = 27; // fixed width for flag column
    let desc_col = inner_width - flag_col; // remaining for description

    // Pad or truncate flag to fixed width
    let flag_padded = if flag.len() < flag_col {
        format!("{}{}", flag, " ".repeat(flag_col - flag.len()))
    } else {
        flag[..flag_col].to_string()
    };

    // Word-wrap description
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

    // Print first line with flag
    if let Some(first) = lines.first() {
        let padding = desc_col.saturating_sub(first.len());
        println!("│ {}{}{} │", flag_padded, first, " ".repeat(padding));
    } else {
        let padding = desc_col;
        println!("│ {}{} │", flag_padded, " ".repeat(padding));
    }

    // Print continuation lines
    let indent = " ".repeat(flag_col);
    for line in lines.iter().skip(1) {
        let padding = desc_col.saturating_sub(line.len());
        println!("│ {}{}{} │", indent, line, " ".repeat(padding));
    }
}

/// Calculate display width accounting for ANSI escape codes
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

/// Print centered text within box width
pub fn print_centered(text: &str) {
    let padding = BOX_WIDTH.saturating_sub(text.len()) / 2;
    print!("{}{}{}\r\n", " ".repeat(padding), text, " ".repeat(BOX_WIDTH - padding - text.len()));
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

// ============================================================================
// Progress Bar
// ============================================================================

/// Render a progress bar inside a box with centered text (3 lines) - raw mode compatible
pub fn progress_bar_box(percent: f32, stats: &str) {
    use std::io::Write;
    let inner_width = BOX_WIDTH - 2; // 72 chars inside the box
    let filled = if percent >= 100.0 {
        inner_width
    } else {
        ((percent / 100.0) * inner_width as f32) as usize
    };

    // Center the stats text (use char count, not byte count)
    let text_chars: Vec<char> = stats.chars().collect();
    let text_len = text_chars.len();
    let padding = if text_len < inner_width {
        (inner_width - text_len) / 2
    } else {
        0
    };

    // Build the content line with text centered
    let mut content: Vec<char> = vec![' '; inner_width];
    for (i, ch) in text_chars.iter().enumerate() {
        if padding + i < inner_width {
            content[padding + i] = *ch;
        }
    }

    // Top border - use quadrant blocks for corners when filled
    if filled > 0 {
        print!("\r▗"); // Lower right quadrant for top-left corner
        print!("{}", "▄".repeat(filled));
    } else {
        print!("\r┌");
    }
    if filled < inner_width {
        print!("{}", "─".repeat(inner_width - filled));
        print!("┐\r\n");
    } else {
        print!("▖\r\n"); // Lower left quadrant for top-right corner
    }

    // Middle - use half blocks for borders to connect with fill
    if filled > 0 {
        print!("\r▐"); // Right half block = fills right side (toward content)
        let filled_str: String = content[..filled].iter().collect();
        print!("\x1b[7m{}\x1b[0m", filled_str); // Reverse video
    } else {
        print!("\r│");
    }
    if filled < inner_width {
        let unfilled_str: String = content[filled..].iter().collect();
        print!("{}", unfilled_str);
        print!("│\r\n");
    } else {
        print!("▌\r\n"); // Left half block = fills left side (toward content)
    }

    // Bottom border - use quadrant blocks for corners when filled
    if filled > 0 {
        print!("\r▝"); // Upper right quadrant for bottom-left corner
        print!("{}", "▀".repeat(filled));
    } else {
        print!("\r└");
    }
    if filled < inner_width {
        print!("{}", "─".repeat(inner_width - filled));
        print!("┘\r\n");
    } else {
        print!("▘\r\n"); // Upper left quadrant for bottom-right corner
    }

    let _ = std::io::stdout().flush();
}

/// Render a countdown bar with bouncing grey spot and centered text (3 lines)
pub fn countdown_bar(spot_pos: usize, text: &str) {
    let inner_width = BOX_WIDTH - 2; // 72 chars inside the box

    // Center the text
    let text_chars: Vec<char> = text.chars().collect();
    let text_len = text_chars.len();
    let padding = if text_len < inner_width {
        (inner_width - text_len) / 2
    } else {
        0
    };

    // Build the content line with text centered and spot
    let mut content: Vec<char> = vec![' '; inner_width];
    for (i, ch) in text_chars.iter().enumerate() {
        if padding + i < inner_width {
            content[padding + i] = *ch;
        }
    }

    // Place the grey spot (if not overlapping text)
    let spot = spot_pos % inner_width;

    // Top border
    print!("\r┌{}┐\r\n", "─".repeat(inner_width));

    // Middle with spot
    print!("\r│");
    for (i, ch) in content.iter().enumerate() {
        if i == spot {
            print!("\x1b[90m█\x1b[0m"); // Grey block
        } else {
            print!("{}", ch);
        }
    }
    print!("│\r\n");

    // Bottom border
    print!("\r└{}┘\r\n", "─".repeat(inner_width));

    let _ = std::io::stdout().flush();
}

// ============================================================================
// Entropy Calculation
// ============================================================================

/// Calculate password entropy in bits
/// entropy = length * log2(charset_size)
pub fn calculate_entropy(password_length: usize, charset_size: usize) -> f64 {
    if charset_size == 0 {
        return 0.0;
    }
    password_length as f64 * (charset_size as f64).log2()
}

/// Get entropy strength description
pub fn entropy_strength(bits: f64) -> &'static str {
    match bits as u32 {
        0..=35 => "Weak",
        36..=59 => "Fair",
        60..=127 => "Strong",
        _ => "Very Strong",
    }
}

// ============================================================================
// Entropy Source Quality
// ============================================================================

/// Get info about the entropy source
pub fn entropy_source_info() -> &'static str {
    if crate::rand::is_urandom_enabled() {
        return "/dev/urandom (32MB pool) - High quality";
    }

    #[cfg(target_arch = "x86_64")]
    { "rdtsc (CPU timestamp counter) - High quality" }

    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    { "pmccntr (ARM cycle counter) - High quality" }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "arm", target_arch = "aarch64")))]
    { "/dev/urandom (32MB pool) - High quality" }
}
