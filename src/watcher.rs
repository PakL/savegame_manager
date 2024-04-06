use crate::*;
use crate::screenshot::SCREENSHOT_STATE;

use std::{path::{Path, PathBuf}, sync::RwLock};
use notify::{RecommendedWatcher, Watcher};

pub static WATCHER_HAS_CHANGES: RwLock<bool> = RwLock::new(false);
pub static WATCHER_LATEST_CHANGE: RwLock<i64> = RwLock::new(0);
pub static WATCHER_PAUSED: RwLock<bool> = RwLock::new(false);

static WATCHER_PATH: RwLock<Option<PathBuf>> = RwLock::new(None);
static WATCHER: RwLock<Option<RecommendedWatcher>> = RwLock::new(None);

struct SavegameSourceWatchEventHandler;

impl notify::EventHandler for SavegameSourceWatchEventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
		let paused = read_rwlock_or(&WATCHER_PAUSED, false);
		if paused {
			return;
		}

        println!("File event: {:?}", event);
        if let Ok(ev) = event {
            let mut only_folders = true;
            for path in ev.paths {
                if path.is_file() {
                    only_folders = false;
                    break;
                }
            }
            if only_folders {
                return;
            }

            let changes_read = read_rwlock_or(&WATCHER_HAS_CHANGES, false);
            if !changes_read {
                write_to_rwlock(&WATCHER_HAS_CHANGES, true);
                write_to_rwlock(&SCREENSHOT_STATE, screenshot::ScreenshotState::Idle);
            }

            write_to_rwlock(&WATCHER_LATEST_CHANGE, chrono::Utc::now().timestamp_millis());
        }
    }
}

pub fn start_watcher(source_path: &String, dest_path: &String) -> bool {
	let src_path = Path::new(source_path.as_str());
	let dst_path = Path::new(dest_path.as_str());

	if let Ok(mut current_path) = WATCHER_PATH.write() {
		if let Ok(mut watcher) = WATCHER.write() {
			if let Some(current_path_path) = current_path.as_ref() {
				if let Some(rec_watch) = watcher.as_mut() {
					rec_watch.unwatch(current_path_path).unwrap_or_default();
				}
			}
		
			if source_path.len() > 0 && dest_path.len() > 0 && src_path.exists() && dst_path.exists() && src_path.is_dir() && dst_path.is_dir() {
				if let Ok(mut rec_watch) = notify::recommended_watcher(SavegameSourceWatchEventHandler) {
					rec_watch.watch(src_path, notify::RecursiveMode::NonRecursive).unwrap_or_default();
					*watcher = Some(rec_watch);
					*current_path = Some(src_path.to_owned());

					true
				} else {
					*watcher = None;
					*current_path = None;

					false
				}
			} else {
				true
			}
		} else {
			false
		}
	} else {
		false
	}
}