use std::{
    fs::{File, OpenOptions},
    path::Path,
    process::exit,
};

use crate::pass::output::with_progress as output_passwords;
use crate::settings::Settings;
use crate::terminal::{clear, reset_terminal};

use super::{
    enter_prompt, get_editable_input, get_numeric_input, print_file_exists, print_help,
    print_main_menu, print_settings_menu,
};

pub fn gen_file_exists_menu(settings: &Settings) -> Option<File> {
    use std::io::Write;

    print_file_exists(&settings.output_file_path);

    loop {
        let answer = get_editable_input("Enter your choice", "")?;

        let choice = answer.trim().to_lowercase();
        if choice == "o" {
            return Some(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&settings.output_file_path)
                    .expect("Failed to open file"),
            );
        } else if choice == "a" {
            return Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&settings.output_file_path)
                    .expect("Failed to open file"),
            );
        } else {
            // Move up 2 lines (to blank line), clear it, print error, move down, clear prompt line
            print!(
                "\x1b[2A\x1b[2K\x1b[31mInvalid choice. Please enter 'a' or 'o'.\x1b[0m\n\x1b[2K"
            );
            let _ = std::io::stdout().flush();
        }
    }
}

pub fn gen_main_menu() {
    reset_terminal();
    clear();

    let mut settings = match Settings::load_from_file() {
        Ok(s) => s,
        Err(e) => {
            println!("Error loading settings: {}", e);
            Settings::default()
        }
    };

    if settings.number_of_passwords > 100 {
        update_settings(&mut settings);
    } else if settings.output_file_path.is_empty() {
        output_passwords(&settings);
    }
    let mut print_invalid = false;

    loop {
        print_main_menu(&mut print_invalid);

        let input = match get_editable_input(enter_prompt(), "") {
            Some(s) => s,
            None => {
                clear();
                continue;
            }
        };

        match input.trim() {
            "" => {
                clear();
                output_passwords(&settings);
                reset_terminal(); // Ensure clean state after password generation
            }
            "1" => {
                // passwords generated in update_settings() after enter is pressed
                update_settings(&mut settings);
            }
            "2" => clear(),
            "3" => {
                clear();
                print_help();
            }
            "4" => {
                clear();
                break;
            }
            _ => {
                clear();
                print_invalid = true;
            }
        }
    }
}

pub fn update_settings(settings: &mut Settings) {
    let (mut print_error, mut last_option, mut error_txt) = (0, String::new(), String::new());

    loop {
        print_settings_menu(settings, print_error, &error_txt);

        let choice = if print_error == 0 || print_error >= 900 {
            let choice = match get_editable_input(enter_prompt(), "") {
                Some(s) => s,
                None => {
                    clear();
                    break; // ESC pressed - return to main menu
                }
            };
            let trim_choice = choice.trim().to_string();
            last_option = trim_choice.clone();
            trim_choice
        } else {
            print_error = 0;
            println!("{}: {}", enter_prompt(), last_option);
            last_option.clone()
        };

        let choice = choice.as_str();

        match choice.parse::<i32>() {
            Ok(num) => {
                if let Break = menu_options(num, &mut print_error, &mut error_txt, settings) {
                    break;
                }
            }
            Err(_) => {
                if let Break = command_options(choice, &mut print_error, &mut error_txt, settings) {
                    break;
                }
            }
        }
    }
}

use LoopAction::*;
pub enum LoopAction {
    Break,
    Continue,
}

fn menu_options(
    choice: i32,
    print_error: &mut i32,
    error_txt: &mut String,
    settings: &mut Settings,
) -> LoopAction {
    match choice {
        1 => {
            // pass length
            if let Some(len) = get_numeric_input("Enter new password length", settings.pass_length)
            {
                settings.pass_length = len;
            }
        }
        2 => {
            // view seeds
            let new_bool = match get_editable_input("Enter 't' or 'f'", "") {
                Some(s) => s,
                None => return Continue,
            };
            match new_bool.trim() {
                "" => return Continue,
                "t" => settings.view_chars_str = true,
                "f" => settings.view_chars_str = false,
                _ => *print_error = 2,
            }
        }
        3 => {
            // num of passwords
            if let Some(num) =
                get_numeric_input("Enter number of passwords", settings.number_of_passwords)
            {
                settings.number_of_passwords = num;
            }
        }

        4 => {
            // special chars
            let chars: String = settings.special_chars.clone().into_iter().collect();
            let new_chars =
                match get_editable_input("Enter new special characters without spaces", &chars) {
                    Some(s) => s,
                    None => return Continue,
                };

            settings.special_chars = new_chars.trim().chars().collect();
        }
        5 => {
            // special char density
            if let Some(d) =
                get_numeric_input("Special char density", settings.special_char_density)
            {
                settings.special_char_density = d;
            }
        }
        6 => {
            // numeric char density
            if let Some(d) =
                get_numeric_input("Numeric char density", settings.numeric_char_density)
            {
                settings.numeric_char_density = d;
            }
        }
        7 => {
            // lowercase char density
            if let Some(d) =
                get_numeric_input("Lowercase char density", settings.lowercase_char_density)
            {
                settings.lowercase_char_density = d;
            }
        }
        8 => {
            // uppercase char density
            if let Some(d) =
                get_numeric_input("Uppercase char density", settings.uppercase_char_density)
            {
                settings.uppercase_char_density = d;
            }
        }
        9 => {
            // output to terminal
            let new_bool = match get_editable_input("Enter 't' or 'f'", "") {
                Some(s) => s,
                None => return Continue,
            };

            match new_bool.trim() {
                "" => return Continue,
                "t" => settings.output_to_terminal = true,
                "f" => settings.output_to_terminal = false,
                _ => *print_error = 2,
            }
        }
        10 => {
            // output file path
            let new_path = match get_editable_input(
                "Enter new .txt output file path",
                &settings.output_file_path,
            ) {
                Some(s) => s,
                None => return Continue,
            };

            let path = match new_path.trim().to_string() {
                path if path.ends_with(".txt") => path,
                path if path.ends_with(".") => format!("{}/rand_pass.txt", path),
                path if path.ends_with("/") => format!("{}rand_pass.txt", path),
                _ => {
                    settings.output_file_path = "".to_string();
                    return Continue;
                }
            };

            match Path::new(path.trim()).parent() {
                Some(_) => (),
                None => {
                    *print_error = 3;
                    return Continue;
                }
            }

            settings.output_file_path = path.trim().to_string();
        }
        11 => {
            // skip countdown
            let new_bool = match get_editable_input("Enter 't' or 'f'", "") {
                Some(s) => s,
                None => return Continue,
            };

            match new_bool.trim() {
                "" => return Continue,
                "t" => settings.skip_countdown = true,
                "f" => settings.skip_countdown = false,
                _ => *print_error = 2,
            }
        }
        12 => {
            // cli command
            let new_command = match get_editable_input("Enter flags and values", "") {
                Some(s) => s,
                None => return Continue,
            };
            settings.cli_command = new_command;
        }
        13 => {
            // entropy source toggle
            if crate::rand::is_urandom_enabled() {
                crate::rand::disable_urandom();
            } else if !crate::rand::enable_urandom() {
                *print_error = 999;
                *error_txt = "/dev/urandom not available on this system".to_string();
            }
        }
        _ => {
            clear();
            *print_error = 998;
        }
    }
    Continue
}

fn command_options(
    choice: &str,
    print_error: &mut i32,
    error_txt: &mut String,
    settings: &mut Settings,
) -> LoopAction {
    // println!("{:?}", choice);
    if choice.is_empty() {
        if settings.output_file_path.is_empty() && !settings.output_to_terminal {
            *print_error = 999;
            *error_txt = "You must output to the terminal or a file.".to_string();
            return Continue; // Stay in settings to show error
        } else {
            // generate passwords
            clear();
            output_passwords(settings);
            return Break;
        }
    }

    if choice == "help" {
        clear();
        print_help();
        return Break;
    }

    match choice.chars().next() {
        Some('s') | Some('e') | Some('r') | Some('f') | Some('d') => {}
        _ => {
            *print_error = 0;
            *error_txt = "Invalid selection".to_string();
            return Continue;
        }
    }

    for ch in choice.chars() {
        match ch {
            's' => {
                // save settings
                if let Err(e) = settings.save_to_file() {
                    *print_error = 1;
                    *error_txt = format!("Error saving settings: {}", e);
                } else {
                    *print_error = 0;
                }
            }
            'e' => {}
            'r' => {
                // load default settings
                *print_error = 0;
                *settings = Settings::default();
            }
            'f' => {
                // load from file
                match Settings::load_from_file() {
                    Ok(s) => {
                        *print_error = 0;
                        *settings = s;
                    }
                    Err(e) => {
                        *print_error = 1;
                        *error_txt = format!("Error loading settings: {}", e);
                    }
                }
            }
            'd' => {
                clear();
                if Path::new(&settings.output_file_path).exists() {
                    let _ = std::fs::remove_file(&settings.output_file_path);
                }
            }
            _ => {
                // invalid input
                clear();
                *print_error = 998;
            }
        }
    }

    if choice.contains("e") {
        clear();
        exit(0);
    }
    Continue
}
