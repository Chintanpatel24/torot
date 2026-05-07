// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    match torot_lib::try_run_cli() {
        Ok(true) => return,
        Ok(false) => {}
        Err(err) => {
            eprintln!("torot cli error: {err}");
            std::process::exit(1);
        }
    }
    torot_lib::run();
}
