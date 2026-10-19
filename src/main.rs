#![windows_subsystem = "windows"]

mod anim;
mod app;
mod config;
mod core;
mod flags;
mod game;
mod geometry;
mod gfx;
mod i18n;
mod icons;
mod install;
mod net;
mod secret;
mod text;
mod theme;
mod ui;




fn main() {
    if let Err(error) = app::App::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
