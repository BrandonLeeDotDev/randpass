use crate::Settings;
use super::terminal::{
    clear, flush, format_number, print_error, box_top, box_line, box_line_center, box_bottom,
    box_opt, print_rule, RESET, UNDERLINE,
};

/// Format special chars as ['a', 'b', ...] with wrapping across multiple lines
fn print_special_chars_wrapped(settings: &Settings) {
    let prefix = "  4) Special Character List: ";
    let indent = "      ";  // continuation line indent
    let max_width = 70; // inner box width

    let mut items: Vec<String> = settings.special_chars.iter()
        .map(|c| format!("'{}'", c))
        .collect();

    if items.is_empty() {
        box_line(&format!("{}[]", prefix));
        return;
    }

    // Build first line
    let mut current_line = format!("{}[", prefix);
    let mut first_item = true;

    while !items.is_empty() {
        let item = &items[0];
        let separator = if first_item { "" } else { ", " };
        let addition = format!("{}{}", separator, item);

        // Check if we need to close on this line (last item)
        let is_last = items.len() == 1;
        let closing = if is_last { "]" } else { "" };

        if current_line.len() + addition.len() + closing.len() <= max_width {
            current_line.push_str(&addition);
            if is_last {
                current_line.push(']');
            }
            items.remove(0);
            first_item = false;
        } else {
            // Line is full, print it and start new line
            box_line(&current_line);
            current_line = indent.to_string();
            first_item = true;
        }
    }

    // Print remaining content
    if !current_line.is_empty() && current_line != indent {
        box_line(&current_line);
    }
}

pub fn enter_prompt() -> &'static str {
    "Enter menu option (or press Enter to generate passwords)"
}

pub fn print_help() {
    box_top("Randpass");
    box_line_center("High-entropy password generator");
    box_line("");
    box_line("MODES:");
    box_line("  1) Interactive: Run without arguments. Opens a TUI menu to");
    box_line("     configure settings and generate passwords.");
    box_line("  2) Client: Pass flags directly (e.g., -l 20 -n 5) to generate");
    box_line("     passwords without the menu.");
    box_line("  3) Command: Use -c to save flags as defaults. Future runs of");
    box_line("     `randpass` will use those flags automatically. Clear with");
    box_line("     `randpass -c`.");
    box_line("");
    box_line("USAGE:");
    box_line("  randpass [OPTIONS]");
    box_line("");
    box_line("OPTIONS:");
    box_line(" Password:");
    box_opt("  -l, --length <N>", "Characters per password (default: 74)");
    box_opt("  -n, --number <N>", "How many to generate. With --bytes, this is byte count and supports K/M/G suffixes.");
    box_opt("      --hex", "Hex charset only (0-9, a-f)");
    box_opt("      --no-special", "Alphanumeric only, no special characters");
    box_opt("      --special <CHARS>", "Override special character set");
    box_line("");
    box_line(" Output:");
    box_opt("  -o, --output [FILE]", "Write to file (default: rand_pass.txt)");
    box_opt("  -b, --board", "Copy to clipboard instead of printing");
    box_opt("  -q, --quiet", "Suppress all output except passwords/bytes");
    box_line("");
    box_line(" Settings:");
    box_opt("  -c, --command [FLAGS]", "Save flags as defaults. Run alone to clear.");
    box_opt("  -d, --default", "Use default settings");
    box_opt("  -s, --saved", "Use saved settings from config file");
    box_line("");
    box_line(" Entropy:");
    box_opt("  -u, --urandom", "Use /dev/urandom pool instead of hardware");
    box_opt("      --bytes", "Output raw bytes. Use -n for limit, -o for file.");
    box_line("");
    box_line(" Info:");
    box_opt("  -h, --help", "Display this help message");
    box_opt("  -v, --version", "Display version");
    box_line("");
    box_line("EXAMPLES:");
    box_line("  randpass                 Interactive or command mode (if set)");
    box_line("  randpass -l 16           One password, 16 characters");
    box_line("  randpass -l 20 -n 3      Three passwords, 20 characters each");
    box_line("  randpass -l 32 --hex     32-character hex string");
    box_line("  randpass --no-special    Alphanumeric only");
    box_line("  randpass -c -l 20        Save -l 20 as default");
    box_line("  randpass --bytes -n 1M   1MB of random bytes to stdout");
    box_line("");
    box_bottom();
    println!();
}

pub fn print_file_exists(file_name: &str) {
    print_error(&format!("File {file_name} already exists."));
    println!();
    box_top("");
    box_line_center("a) append | o) overwrite");
    box_bottom();
    println!();
    flush();
}

pub fn print_main_menu(print_invalid: &mut bool) {
    box_top("Main Menu");
    box_line("");
    box_line("  1) settings");
    box_line("  2) clear");
    box_line("  3) help");
    box_line("  4) quit");
    box_line("");
    box_bottom();

    // Error message (or blank line if no error)
    if *print_invalid {
        print_error("Invalid option.");
        *print_invalid = false;
    } else {
        println!();
    }
    flush();
}

pub fn print_settings_menu(settings: &Settings, print_error_code: i32, error_txt: &String) {
    clear();
    box_top("Settings Menu");
    box_line_center("Esc/CTRL+Q: cancel | CTRL+U: clear input");
    box_line("");

    // General section
    box_line(&format!("{UNDERLINE}General{RESET}:"));
    box_line(&format!("  1) Password Length: {}", format_number(settings.pass_length)));
    box_line(&format!("  2) View Seed Strings: {}", settings.view_chars_str));
    box_line(&format!("  3) Number of Passwords: {}", format_number(settings.number_of_passwords)));
    print_special_chars_wrapped(settings);

    // Character Density section
    box_line("");
    box_line(&format!("{UNDERLINE}Character Density Multiplier{RESET}:"));
    box_line(&format!("  5) Special: {}", format_number(settings.special_char_density)));
    box_line(&format!("  6) Numeric: {}", format_number(settings.numeric_char_density)));
    box_line(&format!("  7) Lowercase: {}", format_number(settings.lowercase_char_density)));
    box_line(&format!("  8) Uppercase: {}", format_number(settings.uppercase_char_density)));

    // Output section
    box_line("");
    box_line(&format!("{UNDERLINE}Output{RESET}:"));
    box_line(&format!("  9) Password(s) to terminal: {}", settings.output_to_terminal));
    box_line(&format!("  10) Password output file path: {}", settings.output_file_path));
    box_line(&format!("  11) Skip Pre-Generation Countdown: {}", settings.skip_countdown));
    box_line("      - Occurs when #3 (Number of Passwords) > 100");

    // Command section
    box_line("");
    box_line(&format!("{UNDERLINE}Command on start{RESET}:"));
    box_line(&format!("  12) Command to run with 'randpass': {}", settings.cli_command));
    box_line("      - Ex: -l 22 -n 1 (see help)");

    // Entropy section
    box_line("");
    box_line(&format!("{UNDERLINE}Entropy{RESET}:"));
    box_line(&format!("  13) Source: {}", crate::rand::entropy_source()));

    // Footer
    box_line("");
    print_rule();
    box_line("     r) load defaults  |  f) load saved  |  s) save  |  e) exit");
    box_line("     d) delete output file");
    box_bottom();

    // Error messages (or blank line if no error)
    match print_error_code {
        1 => print_error(&format!("Invalid input, please enter a number up to: {}...", isize::MAX)),
        2 => print_error("Invalid input, please enter 't' or 'f'..."),
        3 => print_error("Invalid input, please enter a valid file path..."),
        998 => print_error("Invalid input, please enter a valid menu option..."),
        999 => print_error(error_txt),
        _ => println!(),
    }
    flush();
}
