use std::env;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use zeroize::Zeroize;
// use std::io::Read;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use crate::clear_terminal;
use crate::menus::terminal::{
    reset_terminal, calculate_entropy, entropy_strength, entropy_source_info,
    RawModeGuard, box_top, box_line, box_bottom, print_centered, progress_bar_box,
    countdown_bar, format_number,
};
use crate::gen_file_exists_menu;
// use crate::gen_file_exists_menu;
use crate::generate_password;
use crate::io;
use crate::OpenOptions;
use crate::Path;
use crate::Settings;
use crate::Write;

fn non_blocking_read(timeout: Duration) -> Option<Event> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if event::poll(timeout).unwrap_or(false) {
            if let Ok(event) = event::read() {
                let _ = tx.send(event);
            }
        }
    });

    rx.recv().ok()
}

/// Draw the generation header with entropy info
fn draw_generation_header(entropy: f64, strength: &str, source: &str, chars: usize, settings: &Settings) {
    box_top("Entropy");
    box_line(&format!("{:.1} bits ({})", entropy, strength));
    box_line(&format!("Source: {} • Charset: {} chars", source, chars));
    box_bottom();
    println!();

    // Only show interrupt hint when >100 passwords (when interrupt is active)
    if settings.number_of_passwords > 100 {
        print_centered("[Esc/Ctrl+C] to interrupt");
        println!();
    }

    if !settings.output_to_terminal {
        let full_path = std::fs::canonicalize(&settings.output_file_path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| settings.output_file_path.clone());
        print_centered(&format!("Output: {}", full_path));
        println!();
    }
}

/// Calculate charset size based on settings
fn charset_size(settings: &Settings) -> usize {
    let mut size = 0;
    size += 26 * settings.lowercase_char_density;  // a-z
    size += 26 * settings.uppercase_char_density;  // A-Z
    size += 10 * settings.numeric_char_density;    // 0-9
    size += settings.special_chars.len() * settings.special_char_density;
    size
}

pub fn output_passwords(settings: &Settings) {
    // Reset terminal state to fix any raw mode issues
    reset_terminal();

    // Calculate entropy info
    let chars = charset_size(settings);
    let entropy = calculate_entropy(settings.pass_length, chars);
    let strength = entropy_strength(entropy);
    let source = entropy_source_info().split(" (").next().unwrap_or("unknown");

    // Display initial header
    clear_terminal();
    draw_generation_header(entropy, strength, source, chars, settings);

    let mut file = get_file(&settings);

    if let None = file {
        if !settings.output_file_path.is_empty() {
            clear_terminal();
            return;
        }
    }

    // Redraw clean header after file dialog (if there was one)
    if !settings.output_to_terminal && !settings.output_file_path.is_empty() {
        clear_terminal();
        draw_generation_header(entropy, strength, source, chars, settings);
    }

    let (tx, rx) = mpsc::channel::<KeyCode>();
    let (close_tx, close_rx) = mpsc::channel();

    // Enable raw mode for proper Esc key capture - crossterm events need raw mode
    let _raw_guard = RawModeGuard::new().ok();

    thread::spawn(move || {
        let timeout = Duration::from_millis(1);
        loop {
            if let Ok(_) | Err(TryRecvError::Disconnected) = close_rx.try_recv() {
                break;
            }

            if let Some(event) = non_blocking_read(timeout) {
                if let Event::Key(key_event) = event {
                    let is_ctrl_c = key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL);
                    if is_ctrl_c {
                        let _ = tx.send(KeyCode::Esc); // Treat Ctrl+C as Esc
                        break; // Stop listening on abort
                    } else if key_event.code == KeyCode::Esc {
                        let _ = tx.send(KeyCode::Esc);
                        break; // Stop listening on abort
                    } else if key_event.code == KeyCode::Enter {
                        let _ = tx.send(KeyCode::Enter);
                        // Don't break - keep listening for Esc during generation
                    }
                }
            }
        }
    });

    // sleep(Duration::from_millis(1));

    if !settings.skip_countdown && settings.number_of_passwords > 100 {
        use crate::rand::Rand;

        print!("\x1b[?25l"); // Hide cursor during countdown
        io::stdout().flush().expect("Failed to flush stdout");

        // Print initial bar placeholder (3 lines)
        println!();
        println!();
        println!();

        let start = Instant::now();
        let total_duration = Duration::from_secs(10);

        // Bouncing spot state
        let mut spot_pos: i32 = (Rand::get() as i32).abs() % 72;
        let mut direction: i32 = if Rand::get() % 2 == 0 { 1 } else { -1 };

        let mut aborted = false;
        while start.elapsed() < total_duration {
            match rx.try_recv() {
                Ok(KeyCode::Enter) => break, // Start now
                Ok(_) => {
                    // Esc/Ctrl+C - abort
                    aborted = true;
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    aborted = true;
                    break;
                }
                Err(TryRecvError::Empty) => {} // No input, continue
            }

            let remaining = total_duration.saturating_sub(start.elapsed());
            let secs_left = remaining.as_secs() + 1;
            let text = format!("Starting in {:02}s... [Enter] Start Now", secs_left);

            // Move up 3 lines and redraw
            print!("\x1b[3A");
            countdown_bar(spot_pos as usize, &text);

            // Move spot and bounce off edges
            spot_pos += direction;
            if spot_pos <= 0 {
                spot_pos = 0;
                direction = 1;
            } else if spot_pos >= 71 {
                spot_pos = 71;
                direction = -1;
            }

            sleep(Duration::from_millis(100));
        }

        // Clear the 3 bar lines
        print!("\x1b[3A\x1b[J");
        io::stdout().flush().expect("Failed to flush stdout");

        if aborted {
            let _ = close_tx.send(());
            print!("\x1b[?25h"); // Show cursor
            io::stdout().flush().expect("Failed to flush stdout");
            reset_terminal();
            println!();
            box_top("Cancelled");
            box_line("Generation aborted by user");
            box_bottom();
            println!();
            return;
        }
    }

    // Clear header when printing to terminal
    if settings.output_to_terminal {
        clear_terminal();
    }

    let start_time = Instant::now();

    // Print placeholder lines for progress display (file output only)
    if !settings.output_to_terminal {
        print!("\x1b[?25l"); // Hide cursor
        io::stdout().flush().expect("Failed to flush stdout");
        println!();  // Box top
        println!();  // Box content (progress with stats)
        println!();  // Box bottom
    }

    for n in 0..settings.number_of_passwords {
        if settings.number_of_passwords > 100 {
            // Only Esc/Ctrl+C interrupt, not Enter
            let should_interrupt = match rx.try_recv() {
                Ok(KeyCode::Esc) => true,
                Err(TryRecvError::Disconnected) => true,
                _ => false,
            };
            if should_interrupt {
                let printed = if !settings.output_to_terminal {
                    clear_terminal();
                    "".to_owned()
                } else {
                    if n > 10 {
                        clear_last_n_lines(10);
                        format!(", printed {}", n - 8)
                    } else {
                        "".to_owned()
                    }
                };
                // Stop the listener thread before returning
                let _ = close_tx.send(());
                print!("\x1b[?25h"); // Show cursor
                io::stdout().flush().expect("Failed to flush stdout");
                reset_terminal();

                println!();
                box_top("Interrupted");
                box_line(&format!("{n} password(s) generated in {}ms{}",
                    start_time.elapsed().as_millis(), printed));
                box_bottom();
                println!();
                return;
            }
        }

        let mut password = generate_password(&settings);
        pass_to_file(&password, &mut file);

        if settings.output_to_terminal {
            // Write directly to avoid format buffer allocations
            let stdout = io::stdout();
            let mut out = stdout.lock();
            let _ = out.write_all(b"\r");
            let _ = out.write_all(password.as_bytes());
            let _ = out.write_all(b"\r\n");
            drop(out);
        } else {
            let num_of_passwords = settings.number_of_passwords as f32;
            let current_percent_done = ((n + 1) as f32 / num_of_passwords) * 100.0;
            let elapsed_time = start_time.elapsed();
            let average_time_per_password =
                (elapsed_time.as_millis() as f32) / 1000.0 / (n as f32 + 1.0);
            let passwords_left = (settings.number_of_passwords as f32) - (n as f32 + 1.0);
            let estimated_time_remaining = average_time_per_password * passwords_left;
            // Stats line: count, percentage and ETA
            let stats = format!("{} of {} • {:.1}% • ETA: {:.1}s",
                format_number(n + 1),
                format_number(settings.number_of_passwords),
                current_percent_done,
                estimated_time_remaining);
            // Move up 3 lines, print progress box with stats inside
            print!("\x1b[3A"); // Move up 3 lines
            progress_bar_box(current_percent_done, &stats);
            io::stdout().flush().expect("Failed to flush stdout");
        }

        // Securely zero password memory after use
        password.zeroize();
    }

    let _ = close_tx.send(());

    // Drop raw mode guard explicitly before printing
    drop(_raw_guard);

    // Show cursor and ensure terminal is in good state
    print!("\x1b[?25h");
    io::stdout().flush().expect("Failed to flush stdout");
    reset_terminal();

    if !settings.output_to_terminal {
        clear_terminal();
    }

    println!();
    box_top("Complete");
    box_line(&format!("{} password(s) generated in {}ms",
        settings.number_of_passwords,
        start_time.elapsed().as_millis()));
    if !settings.output_to_terminal {
        let full_path = std::fs::canonicalize(&settings.output_file_path)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| settings.output_file_path.clone());
        box_line(&format!("Output: {}", full_path));
    }
    box_bottom();
    println!();
}

fn clear_last_n_lines(n: usize) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for _ in 0..n {
        // Move up one line
        let _ = handle.write_all(b"\x1b[1A");
        let _ = handle.write_all(b"\x1b[2K");
    }

    let _ = handle.flush();
}

fn get_file(settings: &Settings) -> Option<File> {
    if !settings.output_file_path.is_empty() {
        let path = Path::new(&settings.output_file_path);
        if path.exists() {
            gen_file_exists_menu(settings)
        } else {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    match std::fs::create_dir_all(parent) {
                        Ok(_) => (),
                        Err(_) => {
                            return None;
                        }
                    }
                }
            }
            Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&settings.output_file_path)
                    .expect("Failed to open file"),
            )
        }
    } else {
        None
    }
}

#[inline]
fn pass_to_file(password: &str, file: &mut Option<File>) {
    use std::io::Write;
    if let Some(file) = file.as_mut() {
        // Write directly to avoid format buffer allocations
        file.write_all(password.as_bytes()).expect("Failed to write to file");
        file.write_all(b"\n").expect("Failed to write to file");
    }
}

pub fn save_settings_to_file(settings: &Settings) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(get_settings_path())?;

    let special_chars_str = settings
        .special_chars
        .iter()
        .map(|&c| {
            match c {
                ',' => "|,".to_string(), // Escape commas
                '|' => "||".to_string(), // Escape the escape character itself
                _ => c.to_string(),
            }
        })
        .collect::<Vec<String>>()
        .join("");

    let settings_data = format!(
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

    file.write_all(settings_data.as_bytes())?;
    Ok(())
}

pub fn load_settings_from_file(settings: &mut Settings) -> std::io::Result<()> {
    let settings_path = get_settings_path();
    if !Path::new(&settings_path).exists() {
        // create the settings directory if it doesn't exist
        if let Some(parent) = Path::new(&settings_path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("Failed to create directory for settings file: {}", e);
                return Ok(());
            }
        }
    }

    let file = OpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .open(get_settings_path())?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    if line.is_empty() {
        // If the file is newly created and empty, save the current settings
        save_settings_to_file(settings)?;
    } else {
        // Parse the settings from the file
        let parts = split_escaped(&line.trim(), ',');

        if parts.len() == 13 {
            settings.pass_length = parts[0].parse().unwrap_or(settings.pass_length);
            settings.number_of_passwords = parts[1].parse().unwrap_or(settings.number_of_passwords);
            settings.skip_countdown = parts[2].parse().unwrap_or(settings.skip_countdown);
            settings.view_chars_str = parts[3].parse().unwrap_or(settings.view_chars_str);
            settings.special_chars = parts[4].chars().map(|c| c).collect::<Vec<char>>();
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
            save_settings_to_file(settings)?;
            load_settings_from_file(settings)?;
        }
    }

    Ok(())
}

#[inline]
fn get_settings_path() -> String {
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".into()); // Default to current directory if HOME is not set
    format!("{}/.config/randpass/settings", home_dir)
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
                // Handle consecutive delimiters
                parts.push(String::new());
            } else {
                parts.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }

    // Add last part if not empty
    if !current.is_empty() || (s.ends_with(delimiter) && escape_next == false) {
        parts.push(current);
    }

    parts
}
