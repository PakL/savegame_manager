#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use savegame_manager::gui;

fn main() {
    gui::start_app();
}