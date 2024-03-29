#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Dependency import order actually matters:
// 1. screenshots
// 2. notify
// 3. nwg
// Or else we get a STATUS_ENTRYPOINT_NOT_FOUND error

mod screenshot;
mod backup;
mod gui;
mod utils;

pub use utils::*;
pub use screenshot::{SCREENSHOT_STATE, SCREENSHOT_ERROR};
pub use backup::{BACKUP_STATE, BACKUP_ERROR, BACKUP_NAME};

fn main() {
    gui::start_app();
}