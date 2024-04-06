use crate::*;

use std::sync::RwLock;
use screenshots::{display_info::DisplayInfo, image::ImageFormat, Screen};

#[derive(Clone)]
pub enum ScreenshotState {
    Idle,
    Busy,
    Finished,
    Error,
}

pub static SCREENSHOT_STATE: RwLock<ScreenshotState> = RwLock::new(ScreenshotState::Idle);
pub static SCREENSHOT_ERROR: RwLock<&'static str> = RwLock::new("No error");

fn take_screenshot() -> Result<(), anyhow::Error> {
    let screens = DisplayInfo::all()?;

    let mut took_screenshot = false;
    for screen in screens {
        if screen.is_primary {
            Screen::new(&screen).capture()?.save_with_format("screenshot.jpg", ImageFormat::Jpeg)?;
            took_screenshot = true;
            break;
        }
    }

    if !took_screenshot {
        return Err(anyhow::anyhow!("No primary display found"));
    }

    Ok(())
}

pub fn create_screenshot() {
    match take_screenshot() {
        Ok(_) => {
            println!("Screenshot saved");
            write_to_rwlock(&SCREENSHOT_STATE, ScreenshotState::Finished);
        },
        Err(err) => {
            println!("Could not create screenshot: {:?}", err);
            write_to_rwlock(&SCREENSHOT_ERROR, "Could not create screenshot");
            write_to_rwlock(&SCREENSHOT_STATE, ScreenshotState::Error);
        }
    }
}