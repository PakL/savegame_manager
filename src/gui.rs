use crate::*;
use crate::backup::{SavegameMeta, look_for_backups, get_meta_for_backup};

use std::{cell::RefCell, fs::File, sync::RwLock, path::{Path, PathBuf}};

use nwg::InsertListViewColumn;
use serde::{Deserialize, Serialize};
use chrono::TimeZone;

use notify::{RecommendedWatcher, Watcher};
use native_windows_gui as nwg;
use native_windows_derive as nwd;
use nwd::NwgUi;
use nwg::{NativeUi, stretch::{geometry::{Size, Rect}, style::{Dimension as D, FlexDirection}}};

const NO_PADDING: Rect<D> = Rect { start: D::Points(0.0), end: D::Points(0.0), top: D::Points(0.0), bottom: D::Points(0.0) };
const DATA_FILE: &str = "savegame_manager.json";

pub static SRC_HAS_CHANGES: RwLock<bool> = RwLock::new(false);
pub static SRC_LATEST_CHANGE: RwLock<i64> = RwLock::new(0);

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct SavegameManagerAppData {
    #[serde(skip)] has_changed: bool,
    source_path: String,
    dest_path: String,
    disable_screenshots: bool,
}

#[derive(Default, NwgUi)]
pub struct SavegameManagerApp {
    data: RefCell<SavegameManagerAppData>,

    watcher_path: RefCell<Option<PathBuf>>,
    watcher: RefCell<Option<RecommendedWatcher>>,

    #[nwg_control(size: (800, 600), title: "Savegame Manager", flags: "MAIN_WINDOW")]
    #[nwg_events( OnWindowClose: [SavegameManagerApp::exit] )]
    window: nwg::Window,

    #[nwg_control(parent: window, interval: std::time::Duration::from_millis(500), max_tick: Some(1), active: true)]
    #[nwg_events(OnTimerTick: [SavegameManagerApp::timer_tick])]
    timer: nwg::AnimationTimer,

    #[nwg_layout(parent: window, flex_direction: FlexDirection::Column)]
    layout: nwg::FlexboxLayout,

    // Source folder selection
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    source_frame: nwg::Frame,

    #[nwg_layout(parent: source_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    source_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: source_frame, text: "Source:")]
    #[nwg_layout_item(layout: source_layout, size: Size { width: D::Points(100.0), height: D::Auto })]
    source_label: nwg::Label,

    #[nwg_control(parent: source_frame, text: "Select source folder")]
    #[nwg_layout_item(layout: source_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::select_folder(SELF, CTRL)])]
    source_button: nwg::Button,

    // Destination folder selection
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    dest_frame: nwg::Frame,

    #[nwg_layout(parent: dest_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    dest_layout: nwg::FlexboxLayout,
    #[nwg_control(parent: dest_frame, text: "Backup:")]
    #[nwg_layout_item(layout: dest_layout, size: Size { width: D::Points(100.0), height: D::Auto })]
    dest_label: nwg::Label,

    #[nwg_control(parent: dest_frame, text: "Select backup folder")]
    #[nwg_layout_item(layout: dest_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::select_folder(SELF, CTRL)])]
    dest_button: nwg::Button,


    #[nwg_control(parent: window, text: "Make screenshots when creating backup", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::checkbox_click])]
    screenshots_check: nwg::CheckBox,

    #[nwg_control(parent: window)]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    savegame_list: SavegameListView,
}


struct SavegameSourceWatchEventHandler;

impl notify::EventHandler for SavegameSourceWatchEventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
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

            let changes_read = read_rwlock_or(&SRC_HAS_CHANGES, false);
            if !changes_read {
                write_to_rwlock(&SRC_HAS_CHANGES, true);
                write_to_rwlock(&SCREENSHOT_STATE, 0);
            }

            write_to_rwlock(&SRC_LATEST_CHANGE, chrono::Utc::now().timestamp_millis());
        }
    }
}

impl SavegameManagerApp {
    fn exit(&self) {
        let data = self.data.borrow();

        if data.has_changed {
            if let Ok(file) = File::create(DATA_FILE) {
                serde_json::to_writer_pretty(file, &*data).unwrap_or_default();
            }
        }

        nwg::stop_thread_dispatch();
    }

    fn start_watcher(&self) {
        let data = self.data.borrow();
        let src_path = Path::new(data.source_path.as_str());
        let dst_path = Path::new(data.dest_path.as_str());

        if data.source_path.len() > 0 && data.dest_path.len() > 0 && src_path.exists() && dst_path.exists() && src_path.is_dir() && dst_path.is_dir() {
            let mut current_path = self.watcher_path.borrow_mut();
            let mut watcher = self.watcher.borrow_mut();

            if let Some(current_path_path) = current_path.as_ref() {
                if let Some(rec_watch) = watcher.as_mut() {
                    rec_watch.unwatch(current_path_path).unwrap_or_default();
                }
            }
            let rec_watch_result = notify::recommended_watcher(SavegameSourceWatchEventHandler);
            if let Ok(mut rec_watch) = rec_watch_result {
                rec_watch.watch(src_path, notify::RecursiveMode::NonRecursive).unwrap_or_default();
                *watcher = Some(rec_watch);
                *current_path = Some(src_path.to_owned());
            } else {
                *watcher = None;
                *current_path = None;
            }
        }
    }

    fn tick_screenshot(&self) -> bool {
        if !self.data.borrow().disable_screenshots {
            let state = read_rwlock_or(&SCREENSHOT_STATE, 0);
            match state {
                0 => {
                    println!("Taking screenshot");
                    write_to_rwlock(&SCREENSHOT_STATE, 1);
                    std::thread::spawn(super::screenshot::create_screenshot);
                    true
                },
                1 => {
                    true
                },
                3 => {
                    let error = read_rwlock_or(&SCREENSHOT_ERROR, "Unknown error occured while taking screenshot");
                    nwg::modal_error_message(&self.window.handle, "Screenshot error", error);
                    false
                },
                _ => {
                    false
                },
            }
        } else {
            false
        }
    }

    fn tick_backup(&self, wait_for_screenshot: bool) {
        match read_rwlock_or(&BACKUP_STATE, 0) {
            0 => {
                let now = chrono::Utc::now().timestamp_millis();
                let last_change = read_rwlock_or(&SRC_LATEST_CHANGE, now);
                if now - last_change > 1_000 && !wait_for_screenshot {
                    println!("Creating backup");
                    write_to_rwlock(&BACKUP_STATE, 1);
                    let src_path = self.data.borrow().source_path.clone();
                    let dst_path = self.data.borrow().dest_path.clone();
                    let copy_screenshot = !self.data.borrow().disable_screenshots;
                    std::thread::spawn(move || super::backup::create_backup(&src_path, &dst_path, &copy_screenshot));
                }
            },
            1 => {},
            _ => {
                let error = read_rwlock_or(&BACKUP_ERROR, String::new());
                if error.len() > 0 {
                    nwg::modal_error_message(&self.window.handle, "Backup error", error.as_str());
                } else {
                    let backup_name = read_rwlock_or(&BACKUP_NAME, String::new());
                    if backup_name.len() > 0 {
                        let dst_path = self.data.borrow().dest_path.clone();
                        match get_meta_for_backup(&dst_path, &backup_name) {
                            Ok(meta) => {
                                self.savegame_list.unshift_savegame(meta);
                            },
                            Err(err) => {
                                nwg::modal_error_message(&self.window, "Backup error", format!("Error reading backup meta: {:?}", err).as_str());
                            }
                        }
                    }
                }

                println!("Finishing up");
                write_to_rwlock(&BACKUP_STATE, 0);
                write_to_rwlock(&SRC_HAS_CHANGES, false);
            }
        }
    }

    fn timer_tick(&self) {
        let changes = read_rwlock_or(&SRC_HAS_CHANGES, false);
        if changes {
            self.tick_backup(self.tick_screenshot());
        }

        self.timer.start();
    }

    fn refresh_backup_list(&self) {
        let dst_path = self.data.borrow().dest_path.clone();
        match look_for_backups(&dst_path) {
            Ok(backups) => {
                self.savegame_list.clear_list();
                self.savegame_list.set_redraw(false);
                for backup in backups {
                    self.savegame_list.push_savegame(backup);
                }

                self.savegame_list.update_list();
            },
            Err(err) => {
                nwg::modal_error_message(&self.window, "Backup error", format!("Error reading backups: {:?}", err).as_str());
            }
        }
    }

    fn load_data(&self) {
        let mut data = self.data.borrow_mut();

        match File::open(DATA_FILE) {
            Ok(file) => {
                match serde_json::from_reader::<File, SavegameManagerAppData>(file) {
                    Ok(json) => {
                        *data = json;
                        if data.source_path.len() > 0 {
                            self.source_button.set_text(&data.source_path);
                        }
                        if data.dest_path.len() > 0 {
                            self.dest_button.set_text(&data.dest_path);
                        }
                        self.screenshots_check.set_check_state(if data.disable_screenshots { nwg::CheckBoxState::Unchecked } else { nwg::CheckBoxState::Checked });
                    },
                    Err(err) => {
                        nwg::modal_error_message(&self.window, "Config file error", format!("Unable to parse config file. {:?}", err).as_str());
                    }
                }
            },
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::NotFound => { println!("No config file found"); },
                    std::io::ErrorKind::PermissionDenied => { nwg::modal_error_message(&self.window, "Config file error", "Unable to open config file. Permission was denied."); },
                    e => { nwg::modal_error_message(&self.window, "Config file error", format!("An unusual error occured trying to open config file. {:?}", e).as_str()); },
                }
            }
        }

        drop(data);
        self.refresh_backup_list();

    }

    fn open_dialog(title: &str, handle: &nwg::ControlHandle) -> Result<nwg::FileDialog, nwg::NwgError> {
        let mut dialog = nwg::FileDialog::default();
        nwg::FileDialog::builder()
            .action(nwg::FileDialogAction::OpenDirectory)
            .title(title)
            .build(&mut dialog).expect("Failed to create file dialog");

        if dialog.run(Some(handle)) {
            Ok(dialog)
        } else {
            Err(nwg::NwgError::Unknown)
        }
    }

    fn select_folder(&self, button: &nwg::Button) {
        let title = if button == &self.source_button { "Select source folder" } else { "Select backup folder" };

        if let Ok(dialog) = SavegameManagerApp::open_dialog(title, &self.window.handle) {
            match dialog.get_selected_item() {
                Ok(path) =>
                    if let Ok(path_string) = path.into_string() {
                        button.set_text(path_string.as_str());

                        let mut data = self.data.borrow_mut();
                        if button == &self.source_button {
                            data.source_path = path_string;
                        } else {
                            data.dest_path = path_string;
                        }
                        data.has_changed = true;
                        drop(data);

                        if button == &self.source_button {
                            self.start_watcher();
                        } else {
                            self.refresh_backup_list();
                        }
                    }
                Err(_) => {}
            }
        }
    }

    fn checkbox_click(&self) {
        let mut data = self.data.borrow_mut();
        data.disable_screenshots = match self.screenshots_check.check_state() {
            nwg::CheckBoxState::Unchecked => true,
            _ => false,
        };
        data.has_changed = true;
    }
}


#[derive(Default)]
struct SavegameListView {
    base: nwg::ListView,
    data: RefCell<Vec<SavegameMeta>>,
}

nwg::subclass_control!(SavegameListView, ListView, base);

impl SavegameListView {
    fn builder() -> SavegameListViewBuilder {
        SavegameListViewBuilder {
            list_builder: nwg::ListView::builder()
                .list_style(nwg::ListViewStyle::Detailed)
                .flags(nwg::ListViewFlags::VISIBLE | nwg::ListViewFlags::SINGLE_SELECTION | nwg::ListViewFlags::ALWAYS_SHOW_SELECTION)
                .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID | nwg::ListViewExFlags::AUTO_COLUMN_SIZE)
        }
    }

    fn prepare_list(&self) {
        self.base.insert_column(nwg::InsertListViewColumn { index: Some(0), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(300), text: Some("Name".to_owned()) });
        self.base.insert_column(nwg::InsertListViewColumn { index: Some(1), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(300), text: Some("Date".to_owned()) });
        self.base.insert_column(nwg::InsertListViewColumn { index: Some(2), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(300), text: Some("Checksum".to_owned()) });

        let row = [
            nwg::InsertListViewItem { column_index: 0, index: Some(0), text: Some("Name".to_owned()), image: None },
            nwg::InsertListViewItem { column_index: 1, index: Some(0), text: Some("Date".to_owned()), image: None },
            nwg::InsertListViewItem { column_index: 2, index: Some(0), text: Some("Checksum".to_owned()), image: None },
        ];
        self.base.insert_items(&row);
    }

    fn clear_list(&self) {
        self.base.set_redraw(false);

        let data = self.data.borrow();
        let data_len = data.len();
        drop(data);
        for _ in 0..data_len {
            self.remove_savegame(0);
        }

        self.base.set_redraw(true);
    }

    fn push_savegame(&self, meta: SavegameMeta) {
        let mut data = self.data.borrow_mut();
        data.push(meta.clone());

        let index = data.len();

        let save_timestamp = match chrono::Local.timestamp_millis_opt(meta.date) {
            chrono::offset::LocalResult::Single(ts) => ts,
            chrono::offset::LocalResult::Ambiguous(ts, _) => ts,
            _ => chrono::Local::now(),
        };

        let row = [
            nwg::InsertListViewItem { column_index: 0, index: Some(index as i32), text: Some(meta.name), image: None },
            nwg::InsertListViewItem { column_index: 1, index: Some(index as i32), text: Some(save_timestamp.format("%c").to_string()), image: None },
            nwg::InsertListViewItem { column_index: 2, index: Some(index as i32), text: Some(meta.checksums.iter().map(|cs| format!("{}: {}", cs.0, cs.1)).collect::<Vec<String>>().join(", ")), image: None },
        ];

        self.base.insert_item(row[0].clone());
        self.base.update_item(index, row[1].clone());
        self.base.update_item(index, row[2].clone());
    }

    fn unshift_savegame(&self, meta: SavegameMeta) {
        let mut data = self.data.borrow_mut();
        data.insert(0, meta.clone());

        let index = data.len();
        drop(data);

        self.base.insert_item(nwg::InsertListViewItem { column_index: 0, index: Some(index as i32), text: Some("-".to_owned()), image: None });

        self.update_list();
    }

    fn update_list(&self) {
        let mut data = self.data.borrow_mut();
        data.sort_by(|a, b| b.date.cmp(&a.date));

        self.base.set_redraw(false);

        let mut index = 1;
        for row in &*data {
            let save_timestamp = match chrono::Local.timestamp_millis_opt(row.date) {
                chrono::offset::LocalResult::Single(ts) => ts,
                chrono::offset::LocalResult::Ambiguous(ts, _) => ts,
                _ => chrono::Local::now(),
            };

            self.base.update_item(index, nwg::InsertListViewItem { column_index: 0, index: Some(index as i32), text: Some(row.name.clone()), image: None });
            self.base.update_item(index, nwg::InsertListViewItem { column_index: 1, index: Some(index as i32), text: Some(save_timestamp.format("%c").to_string()), image: None });
            self.base.update_item(index, nwg::InsertListViewItem { column_index: 2, index: Some(index as i32), text: Some(row.checksums.iter().map(|cs| format!("{}: {}", cs.0, cs.1)).collect::<Vec<String>>().join(", ")), image: None });

            index += 1;
        }

        self.base.set_redraw(true);
    }

    fn remove_savegame(&self, index: usize) {
        let mut data = self.data.borrow_mut();
        data.remove(index);

        self.base.remove_item(index + 1);
    }
}

struct SavegameListViewBuilder {
    list_builder: nwg::ListViewBuilder,
}

impl SavegameListViewBuilder {
    pub fn parent<C: Into<nwg::ControlHandle>>(mut self, p: C) -> SavegameListViewBuilder {
        self.list_builder = self.list_builder.parent(p);
        self
    }

    pub fn build(self, list: &mut SavegameListView) -> Result<(), nwg::NwgError> {
        self.list_builder.build(&mut list.base)?;
        list.prepare_list();
        Ok(())
    }
}

pub fn start_app() {
    nwg::init().expect("Failed to init native Windows gui");

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI")
        .size(16)
        .weight(300)
        .build(&mut font).expect("Failed to build default font");
    nwg::Font::set_global_default(Some(font));
    let app = SavegameManagerApp::build_ui(Default::default()).expect("Failed to build ui");
    app.load_data();
    app.start_watcher();

    // make window visible after construction is done to avoid render glitches
    app.window.set_visible(true);
    nwg::dispatch_thread_events();
}