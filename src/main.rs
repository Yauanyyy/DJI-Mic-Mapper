#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod keymap;
mod logger;

#[cfg(windows)]
mod win_app;

#[cfg(windows)]
fn main() {
    if let Err(error) = win_app::run() {
        let message = format!("DJI Mic Mapper failed to start:\n{error}");
        logger::log(logger::Level::Error, &message);
        win_app::show_fatal_error(&message);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("DJI Mic Mapper only supports Windows.");
}
