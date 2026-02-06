//! Password output with TUI progress display.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use zeroize::Zeroize;

use crate::settings::Settings;
use crate::terminal::{
    RawModeGuard, box_bottom, box_line, box_top, calculate_entropy, clear, countdown_bar,
    entropy_source_info, entropy_strength, format_number, print_centered, progress_bar_box,
    reset_terminal,
};
use crate::tui::gen_file_exists_menu;

use super::{charset, generate, generate_from_charset};

fn non_blocking_read(timeout: Duration) -> Option<Event> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        if event::poll(timeout).unwrap_or(false)
            && let Ok(event) = event::read()
        {
            let _ = tx.send(event);
        }
    });

    rx.recv().ok()
}

fn draw_header(entropy: f64, strength: &str, source: &str, chars: usize, settings: &Settings) {
    box_top("Entropy");
    box_line(&format!("{:.1} bits ({})", entropy, strength));
    box_line(&format!("Source: {} • Charset: {} chars", source, chars));
    box_bottom();
    println!();

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

/// Output passwords with TUI progress bar (for bulk generation).
pub fn with_progress(settings: &Settings) {
    reset_terminal();

    let chars = charset::size(settings);
    let entropy = calculate_entropy(settings.pass_length, chars);
    let strength = entropy_strength(entropy);
    let source = entropy_source_info()
        .split(" (")
        .next()
        .unwrap_or("unknown");

    clear();
    draw_header(entropy, strength, source, chars, settings);

    let mut file = get_file(settings).map(super::SecureBufWriter::new);

    if file.is_none() && !settings.output_file_path.is_empty() {
        clear();
        return;
    }

    if !settings.output_to_terminal && !settings.output_file_path.is_empty() {
        clear();
        draw_header(entropy, strength, source, chars, settings);
    }

    let (tx, rx) = mpsc::channel::<KeyCode>();
    let (close_tx, close_rx) = mpsc::channel();

    let _raw_guard = RawModeGuard::new().ok();

    thread::spawn(move || {
        let timeout = Duration::from_millis(1);
        loop {
            if let Ok(_) | Err(TryRecvError::Disconnected) = close_rx.try_recv() {
                break;
            }

            if let Some(Event::Key(key_event)) = non_blocking_read(timeout) {
                let is_ctrl_c = key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL);
                if is_ctrl_c || key_event.code == KeyCode::Esc {
                    let _ = tx.send(KeyCode::Esc);
                    break;
                } else if key_event.code == KeyCode::Enter {
                    let _ = tx.send(KeyCode::Enter);
                }
            }
        }
    });

    if !settings.skip_countdown && settings.number_of_passwords > 100 {
        use crate::rand::Rand;

        print!("\x1b[?25l");
        std::io::stdout().flush().expect("Failed to flush stdout");

        println!();
        println!();
        println!();

        let start = Instant::now();
        let total_duration = Duration::from_secs(10);

        let mut spot_pos: i32 = (Rand::get() as i32).abs() % 72;
        let mut direction: i32 = if Rand::get().is_multiple_of(2) { 1 } else { -1 };

        let mut aborted = false;
        while start.elapsed() < total_duration {
            match rx.try_recv() {
                Ok(KeyCode::Enter) => break,
                Ok(_) => {
                    aborted = true;
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    aborted = true;
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }

            let remaining = total_duration.saturating_sub(start.elapsed());
            let secs_left = remaining.as_secs() + 1;
            let text = format!("Starting in {:02}s... [Enter] Start Now", secs_left);

            print!("\x1b[3A");
            countdown_bar(spot_pos as usize, &text);

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

        print!("\x1b[3A\x1b[J");
        std::io::stdout().flush().expect("Failed to flush stdout");

        if aborted {
            let _ = close_tx.send(());
            print!("\x1b[?25h");
            std::io::stdout().flush().expect("Failed to flush stdout");
            reset_terminal();
            println!();
            box_top("Cancelled");
            box_line("Generation aborted by user");
            box_bottom();
            println!();
            return;
        }
    }

    if settings.output_to_terminal {
        clear();
    }

    let start_time = Instant::now();

    if !settings.output_to_terminal {
        print!("\x1b[?25l");
        std::io::stdout().flush().expect("Failed to flush stdout");
        println!();
        println!();
        println!();
    }

    // Fast path: pre-build charset when not viewing seeds
    let mut base_chars = if !settings.view_chars_str {
        Some(charset::build(settings))
    } else {
        None
    };

    let mut buf = Vec::with_capacity(settings.pass_length + 1);

    for n in 0..settings.number_of_passwords {
        if settings.number_of_passwords > 100 {
            let should_interrupt = matches!(
                rx.try_recv(),
                Ok(KeyCode::Esc) | Err(TryRecvError::Disconnected)
            );
            if should_interrupt {
                let printed = if !settings.output_to_terminal {
                    clear();
                    "".to_owned()
                } else if n > 10 {
                    clear_last_n_lines(10);
                    format!(", printed {}", n - 8)
                } else {
                    "".to_owned()
                };
                let _ = close_tx.send(());
                print!("\x1b[?25h");
                std::io::stdout().flush().expect("Failed to flush stdout");
                reset_terminal();

                println!();
                box_top("Interrupted");
                box_line(&format!(
                    "{n} password(s) generated in {}ms{}",
                    start_time.elapsed().as_millis(),
                    printed
                ));
                box_bottom();
                println!();
                return;
            }
        }

        match &mut base_chars {
            Some(chars) => generate_from_charset(chars, settings.pass_length, &mut buf),
            None => {
                let mut pass = generate(settings);
                buf.clear();
                buf.extend_from_slice(pass.as_bytes());
                pass.zeroize();
            }
        };

        if let Some(ref mut f) = file {
            buf.push(b'\n');
            let _ = f.write_all(&buf);
        }

        if settings.output_to_terminal {
            // Prepend \r, append \r\n for TUI line output
            let mut line = Vec::with_capacity(buf.len() + 3);
            line.push(b'\r');
            line.extend_from_slice(&buf);
            line.extend_from_slice(b"\r\n");
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let _ = out.write_all(&line);
            drop(out);
            line.zeroize();
        } else {
            let num = settings.number_of_passwords as f32;
            let pct = ((n + 1) as f32 / num) * 100.0;
            let elapsed = start_time.elapsed();
            let avg = (elapsed.as_millis() as f32) / 1000.0 / (n as f32 + 1.0);
            let left = num - (n as f32 + 1.0);
            let eta = avg * left;
            let stats = format!(
                "{} of {} • {:.1}% • ETA: {:.1}s",
                format_number(n + 1),
                format_number(settings.number_of_passwords),
                pct,
                eta
            );
            print!("\x1b[3A");
            progress_bar_box(pct, &stats);
            std::io::stdout().flush().expect("Failed to flush stdout");
        }

        buf.zeroize();
    }

    let _ = close_tx.send(());
    drop(_raw_guard);

    print!("\x1b[?25h");
    std::io::stdout().flush().expect("Failed to flush stdout");
    reset_terminal();

    if !settings.output_to_terminal {
        clear();
    }

    println!();
    box_top("Complete");
    box_line(&format!(
        "{} password(s) generated in {}ms",
        settings.number_of_passwords,
        start_time.elapsed().as_millis()
    ));
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
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    for _ in 0..n {
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
            if let Some(parent) = path.parent()
                && !parent.exists()
                && std::fs::create_dir_all(parent).is_err()
            {
                return None;
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

