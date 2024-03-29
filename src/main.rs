#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Dependency import order actually matters:
// 1. chrono
// 2. screenshots
// 3. notify
// 4. nwg
// Or else we get a STATUS_ENTRYPOINT_NOT_FOUND error

mod utils;
mod screenshot;
mod backup;
mod watcher;
mod gui;


pub use utils::*;
pub use screenshot::{SCREENSHOT_STATE, SCREENSHOT_ERROR};
pub use backup::{BACKUP_STATE, BACKUP_ERROR, BACKUP_NAME};
pub use watcher::{WATCHER_HAS_CHANGES, WATCHER_LATEST_CHANGE, WATCHER_PAUSED};

fn main() {
    gui::start_app();
}