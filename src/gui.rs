use crate::*;
use backup::SavegameMeta;

use std::cell::RefMut;
use std::{cell::{Ref, RefCell}, fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use native_windows_gui as nwg;
use native_windows_derive as nwd;
use nwd::NwgUi;
use nwg::{NativeUi, stretch::{geometry::{Size, Rect}, style::{Dimension as D, FlexDirection}}};

const NO_PADDING: Rect<D> = Rect { start: D::Points(0.0), end: D::Points(0.0), top: D::Points(0.0), bottom: D::Points(0.0) };
const PADDING_LEFT: Rect<D> = Rect { start: D::Points(5.0), end: D::Points(0.0), top: D::Points(0.0), bottom: D::Points(0.0) };
const DATA_FILE: &str = "savegame_manager.json";

#[derive(Clone, Serialize, Deserialize)]
pub enum ProfileIntervalUnit {
    Seconds,
    Minutes,
    Hours,
}

impl Default for ProfileIntervalUnit {
    fn default() -> Self { ProfileIntervalUnit::Minutes }
}

impl std::fmt::Display for ProfileIntervalUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Seconds => "seconds",
            Self::Minutes => "minutes",
            Self::Hours => "hours"
        })
    }
}

fn default_true() -> bool { true }

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct SavegameManagerProfile {
    selected: bool,
    name: String,
    src_path: String,
    dst_path: String,
    #[serde(default = "default_true")] screenshots: bool,
    #[serde(default = "default_true")] manual_save_detection: bool,
    auto_saves_max: u16,
    auto_saves_interval: u16,
    auto_saves_interval_unit: ProfileIntervalUnit,
}

impl std::fmt::Display for SavegameManagerProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Default for SavegameManagerProfile {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            name: Default::default(),
            src_path: Default::default(),
            dst_path: Default::default(),
            screenshots: true,
            manual_save_detection: true,
            auto_saves_max: 12,
            auto_saves_interval: 5,
            auto_saves_interval_unit: Default::default()
        }
    }
}

enum RenameMode {
    Backup,
    Profile,
}

impl Default for RenameMode {
    fn default() -> Self { RenameMode::Backup }
}

#[derive(Default, NwgUi)]
pub struct SavegameManagerApp {
    dummy_profile: RefCell<SavegameManagerProfile>,
    profiles_changed: RefCell<bool>,
    selected_backup: RefCell<Option<String>>,
    rename_mode: RefCell<RenameMode>,

    #[nwg_resource(family: "Segoe UI Semibold", size: 16, weight: 400)]
    font_bold: nwg::Font,

    #[nwg_resource(family: "Courier New", size: 14, weight: 400)]
    font_monospace: nwg::Font,


    #[nwg_resource]
    embed: nwg::EmbedResource,

    #[nwg_resource(source_embed: Some(&data.embed), source_embed_str: Some("MAINICON"))]
    window_icon: nwg::Icon,

    #[nwg_resource(source_bin: Some(include_bytes!("../assets/no_screenshot.png")), size: Some((295, 166)))]
    no_screenshot: nwg::Bitmap,

    #[nwg_control(size: (800, 600), title: "Savegame Manager", flags: "MAIN_WINDOW", icon: Some(&data.window_icon))]
    #[nwg_events( OnWindowClose: [SavegameManagerApp::exit] )]
    window: nwg::Window,

    #[nwg_control(parent: window, interval: std::time::Duration::from_millis(500), max_tick: Some(1), active: true)]
    #[nwg_events(OnTimerTick: [SavegameManagerApp::timer_tick])]
    timer: nwg::AnimationTimer,

    #[nwg_layout(parent: window, flex_direction: FlexDirection::Column)]
    layout: nwg::FlexboxLayout,

// region: Profile selection
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    profile_frame: nwg::Frame,

    #[nwg_layout(parent: profile_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    profile_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: profile_frame)]
    #[nwg_layout_item(layout: profile_layout, size: Size { width: D::Auto, height: D::Auto }, margin: Rect { start: D::Points(0.0), top: D::Points(1.0), bottom: D::Points(0.0), end: D::Points(10.0) }, flex_grow: 1.0)]
    #[nwg_events(OnComboxBoxSelection: [SavegameManagerApp::profile_select_change])]
    profile_select: nwg::ComboBox<SavegameManagerProfile>,

    #[nwg_resource]
    tooltip: nwg::Tooltip,

    #[nwg_control(parent: profile_frame, text: "✨")]
    #[nwg_layout_item(layout: profile_layout, size: Size { width: D::Points(30.0), height: D::Auto })]
    #[nwg_events(OnTooltipText: [SavegameManagerApp::tooltip_text(SELF, EVT, EVT_DATA, HANDLE)], OnButtonClick: [SavegameManagerApp::profile_add])]
    profile_add: nwg::Button,

    #[nwg_control(parent: profile_frame, text: "🏷️")]
    #[nwg_layout_item(layout: profile_layout, size: Size { width: D::Points(30.0), height: D::Auto })]
    #[nwg_events(OnTooltipText: [SavegameManagerApp::tooltip_text(SELF, EVT, EVT_DATA, HANDLE)], OnButtonClick: [SavegameManagerApp::profile_rename])]
    profile_rename: nwg::Button,

    #[nwg_control(parent: profile_frame, text: "🗑️")]
    #[nwg_layout_item(layout: profile_layout, size: Size { width: D::Points(30.0), height: D::Auto })]
    #[nwg_events(OnTooltipText: [SavegameManagerApp::tooltip_text(SELF, EVT, EVT_DATA, HANDLE)], OnButtonClick: [SavegameManagerApp::profile_remove])]
    profile_remove: nwg::Button,
// endregion

// region: Source folder selection
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(23.0) })]
    source_frame: nwg::Frame,

    #[nwg_layout(parent: source_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    source_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: source_frame, text: "Source:", v_align: nwg::VTextAlign::Center)]
    #[nwg_layout_item(layout: source_layout, size: Size { width: D::Points(100.0), height: D::Auto })]
    source_label: nwg::Label,

    #[nwg_control(parent: source_frame, text: "Select source folder")]
    #[nwg_layout_item(layout: source_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::select_folder(SELF, CTRL)])]
    source_button: nwg::Button,
// endregion

// region: Destination folder selection
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(23.0) })]
    dest_frame: nwg::Frame,

    #[nwg_layout(parent: dest_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    dest_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: dest_frame, text: "Backup:", v_align: nwg::VTextAlign::Center)]
    #[nwg_layout_item(layout: dest_layout, size: Size { width: D::Points(100.0), height: D::Auto })]
    dest_label: nwg::Label,

    #[nwg_control(parent: dest_frame, text: "Select backup folder")]
    #[nwg_layout_item(layout: dest_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::select_folder(SELF, CTRL)])]
    dest_button: nwg::Button,
// endregion

// region: Checkboxes
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(23.0) })]
    checkboxes_frame: nwg::Frame,

    #[nwg_layout(parent: checkboxes_frame, margin: [0,0,0,0], spacing: 0)]
    checkboxes_layout: nwg::GridLayout,

    #[nwg_control(parent: checkboxes_frame, text: "Make screenshots when creating backup", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: checkboxes_layout, col: 0, row: 0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::screenshots_checkbox_click])]
    screenshots_check: nwg::CheckBox,

    #[nwg_control(parent: checkboxes_frame, text: "Detect difference between auto and manual saves", check_state: nwg::CheckBoxState::Checked)]
    #[nwg_layout_item(layout: checkboxes_layout, col: 1, row: 0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::manual_save_checkbox_click])]
    manual_save_detection_check: nwg::CheckBox,
// endregion

// region: autosave settings
    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(23.0)})]
    autosave_frame: nwg::Frame,

    #[nwg_layout(parent: autosave_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    autosave_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: autosave_frame, text: "Max. autosaves: ", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Center)]
    #[nwg_layout_item(layout: autosave_layout, size: Size { width: D::Points(90.0), height: D::Auto })]
    autosave_lbl_amount: nwg::Label,

    #[nwg_control(parent: autosave_frame, flags: "VISIBLE|NUMBER")]
    #[nwg_layout_item(layout: autosave_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0, margin: PADDING_LEFT)]
    #[nwg_events(OnTextInput: [SavegameManagerApp::autosave_text_input(SELF, HANDLE)])]
    autosave_amount: nwg::TextInput,

    #[nwg_control(parent: autosave_frame, text: "in intervals of: ", h_align: nwg::HTextAlign::Right, v_align: nwg::VTextAlign::Center)]
    #[nwg_layout_item(layout: autosave_layout, size: Size { width: D::Points(90.0), height: D::Auto })]
    autosave_lbl_interval: nwg::Label,

    #[nwg_control(parent: autosave_frame, flags: "VISIBLE|NUMBER")]
    #[nwg_layout_item(layout: autosave_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0, margin: PADDING_LEFT)]
    #[nwg_events(OnTextInput: [SavegameManagerApp::autosave_text_input(SELF, HANDLE)])]
    autosave_interval: nwg::TextInput,

    #[nwg_control(parent: autosave_frame)]
    #[nwg_layout_item(layout: autosave_layout, size: Size { width: D::Points(100.0), height: D::Auto }, margin: PADDING_LEFT)]
    #[nwg_events(OnComboxBoxSelection: [SavegameManagerApp::interval_unit_select_change])]
    autosave_interval_unit: nwg::ComboBox<ProfileIntervalUnit>,
// endregion

    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    savegame_frame: nwg::Frame,

    #[nwg_layout(parent: savegame_frame, flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    savegame_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: savegame_frame)]
    #[nwg_layout_item(layout: savegame_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    #[nwg_events(OnListViewItemChanged: [SavegameManagerApp::show_details], OnKeyRelease: [SavegameManagerApp::key_released(SELF, EVT_DATA)])]
    savegame_list: SavegameListView,

// region: Savegame details
    #[nwg_control(parent: savegame_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: savegame_layout, size: Size { width: D::Points(300.0), height: D::Auto })]
    savegame_detail_frame: nwg::Frame,

    #[nwg_layout(parent: savegame_detail_frame,  flex_direction: FlexDirection::Column, padding: PADDING_LEFT)]
    savegame_detail_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: savegame_detail_frame, text: "Name:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Points(20.0) })]
    savegame_detail_name: nwg::Label,

    #[nwg_control(parent: savegame_detail_frame, text: "-", v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Points(20.0) })]
    savegame_detail_name_content: nwg::Label,
    
    #[nwg_control(parent: savegame_detail_frame, text: "Date:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Points(20.0) })]
    savegame_detail_date: nwg::Label,

    #[nwg_control(parent: savegame_detail_frame, text: "-", v_align: nwg::VTextAlign::Top)]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Points(20.0) })]
    savegame_detail_date_content: nwg::Label,
    
    #[nwg_control(parent: savegame_detail_frame, text: "Checksums:", font: Some(&data.font_bold), v_align: nwg::VTextAlign::Bottom)]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Points(20.0) })]
    savegame_detail_checksums: nwg::Label,

    #[nwg_control(parent: savegame_detail_frame, text: "-", font: Some(&data.font_monospace), flags: "VISIBLE|MULTI_LINE")]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    savegame_detail_checksums_content: nwg::RichLabel,
    
    #[nwg_control(parent: savegame_detail_frame, bitmap: Some(&data.no_screenshot))]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Points(295.0), height: D::Points(166.0) })]
    #[nwg_events(OnImageFrameClick: [SavegameManagerApp::open_screenshot])]
    savegame_detail_screenshot: nwg::ImageFrame,

    #[nwg_control(parent: savegame_detail_frame, flags: "VISIBLE")]
    #[nwg_layout_item(layout: savegame_detail_layout, size: Size { width: D::Auto, height: D::Points(30.0) })]
    savegame_btns_frame: nwg::Frame,

    #[nwg_layout(parent: savegame_btns_frame, margin: [5, 0, 0, 0], spacing: 0)]
    savegame_btns_layout: nwg::GridLayout,

    #[nwg_control(parent: savegame_btns_frame, text: "Load", enabled: false)]
    #[nwg_layout_item(layout: savegame_btns_layout, row: 0, col: 0)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::load_click])]
    savegame_load: nwg::Button,

    #[nwg_control(parent: savegame_btns_frame, text: "Rename", enabled: false)]
    #[nwg_layout_item(layout: savegame_btns_layout, row: 0, col: 1)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::rename_click])]
    savegame_rename: nwg::Button,

    #[nwg_control(parent: savegame_btns_frame, text: "Delete", enabled: false)]
    #[nwg_layout_item(layout: savegame_btns_layout, row: 0, col: 2)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::delete_click])]
    savegame_delete: nwg::Button,
// endregion

// region: rename dialog
    #[nwg_control(parent: Some(&data.window), size: (300, 120), title: "Rename backup", flags: "WINDOW", icon: Some(&data.window_icon))]
    #[nwg_events(OnKeyEsc: [SavegameManagerApp::rename_cancel(SELF, EVT)], OnKeyEnter: [SavegameManagerApp::rename_confirm])]
    rename_dialog: nwg::Window,

    #[nwg_layout(parent: rename_dialog, spacing: 2)]
    rename_layout: nwg::GridLayout,

    #[nwg_control(parent: rename_dialog, text: "", limit: 128)]
    #[nwg_layout_item(layout: rename_layout, row: 0, col: 0, col_span: 2)]
    rename_input: nwg::TextInput,

    #[nwg_control(parent: rename_dialog, text: "Rename")]
    #[nwg_layout_item(layout: rename_layout, row: 1, col: 0)]
    rename_btn: nwg::Button,

    #[nwg_control(parent: rename_dialog, text: "Cancel")]
    #[nwg_layout_item(layout: rename_layout, row: 1, col: 1)]
    #[nwg_events(OnButtonClick: [SavegameManagerApp::rename_cancel(SELF, EVT)])]
    rename_cancel: nwg::Button,
// endregion
}


impl SavegameManagerApp {
    fn get_current_profile(&self) -> Ref<SavegameManagerProfile> {
        if let Some(selection) = self.profile_select.selection() {
            return Ref::map(self.profile_select.collection(), |c| &c[selection]);
        }
        
        self.dummy_profile.borrow()
    }

    fn get_current_profile_mut(&self) -> RefMut<SavegameManagerProfile> {
        if let Some(selection) = self.profile_select.selection() {
            return RefMut::map(self.profile_select.collection_mut(), |c| &mut c[selection]);
        }

        self.dummy_profile.borrow_mut()
    }

    fn exit(&self) {
        let profiles_changed = self.profiles_changed.borrow();

        if *profiles_changed {
            if let Ok(file) = File::create(DATA_FILE) {
                serde_json::to_writer_pretty(file, &*self.profile_select.collection()).unwrap_or_default();
            }
        }

        backup::deal_with_exit_save(&self.get_current_profile().dst_path);

        let screenshot_file = PathBuf::from("screenshot.jpg");
        if screenshot_file.exists() && screenshot_file.is_file() {
            std::fs::remove_file(screenshot_file).unwrap_or_default();
        }


        nwg::stop_thread_dispatch();
    }

    fn start_watcher(&self) {
        let profile = self.get_current_profile();
        if !watcher::start_watcher(&profile.src_path, &profile.dst_path) {
            nwg::modal_error_message(&self.window, "Watcher error", "Could not start folder monitoring");
        }
    }

    fn tick_screenshot(&self) -> bool {
        if self.get_current_profile().screenshots {
            let state = read_rwlock_or(&SCREENSHOT_STATE, screenshot::ScreenshotState::Idle);
            match state {
                screenshot::ScreenshotState::Idle => {
                    println!("Taking screenshot");
                    write_to_rwlock(&SCREENSHOT_STATE, screenshot::ScreenshotState::Busy);
                    std::thread::spawn(screenshot::create_screenshot);
                    true
                },
                screenshot::ScreenshotState::Busy => {
                    true
                },
                screenshot::ScreenshotState::Error => {
                    let error = read_rwlock_or(&SCREENSHOT_ERROR, "Unknown error occured while taking screenshot");
                    nwg::modal_error_message(&self.window.handle, "Screenshot error", error);
                    false
                },
                screenshot::ScreenshotState::Finished => {
                    false
                },
            }
        } else {
            false
        }
    }

    fn finish_up_backup(&self) {
        let error = read_rwlock_or(&BACKUP_ERROR, String::new());
        if error.len() > 0 {
            nwg::modal_error_message(&self.window.handle, "Backup error", error.as_str());
        } else {
            let backup_name = read_rwlock_or(&BACKUP_NAME, String::new());
            if backup_name.len() > 0 {
                let dst_path = self.get_current_profile().dst_path.clone();
                match backup::get_meta_for_backup(&dst_path, &backup_name) {
                    Ok(_) => {
                        self.refresh_backup_list();
                    },
                    Err(err) => {
                        nwg::modal_error_message(&self.window, "Backup error", format!("Error reading backup meta: {:?}", err).as_str());
                    }
                }
            }
        }

        println!("Finishing up");
        write_to_rwlock(&BACKUP_STATE, backup::BackupState::Idle);
        write_to_rwlock(&WATCHER_HAS_CHANGES, false);
    }

    fn start_backup(&self, src_path: String, dst_path: String, copy_screenshot: bool, wait: bool) {
        let mut backup_type: u8 = 0;

        if self.get_current_profile().manual_save_detection {
            let live_hashes = backup::create_hash_list(&src_path);

            if let Some(savegame) = self.savegame_list.get_latest_non_temp() {
                let hash_cmp = backup::hash_list_cmp(&live_hashes, &savegame.checksums);
                match hash_cmp {
                    backup::BackupComparison::CompleteDiff => { backup_type = 0; },
                    backup::BackupComparison::PartialDiff => {
                        let profile = self.get_current_profile();
                        if chrono::Local::now().timestamp_millis() - savegame.date > interval_duration(profile.auto_saves_interval, &profile.auto_saves_interval_unit) {
                            backup_type = 2;
                        } else {
                            backup_type = 1;
                        }
                    },
                    backup::BackupComparison::NoDiff => { backup_type = 3; },
                }
            }
        }

        let autosave_max = self.get_current_profile().auto_saves_max.clone();
        let fun = move || {
            match backup_type {
                0 => backup::create_savetokeep(&src_path, &dst_path, &copy_screenshot),
                1 => backup::create_tempsave(&src_path, &dst_path, &copy_screenshot),
                2 => backup::create_autosave(&src_path, &dst_path, &copy_screenshot, &autosave_max),
                _ => {}
            }
        };

        if wait {
            fun();
        } else {
            std::thread::spawn(fun);
        }
    }

    fn tick_backup(&self, wait_for_screenshot: bool) {
        match read_rwlock_or(&BACKUP_STATE, backup::BackupState::Idle) {
            backup::BackupState::Idle => {
                let now = chrono::Utc::now().timestamp_millis();
                let last_change = read_rwlock_or(&WATCHER_LATEST_CHANGE, now);
                if now - last_change > 1_000 && !wait_for_screenshot {
                    println!("Creating backup");
                    write_to_rwlock(&BACKUP_STATE, backup::BackupState::Busy);
                    let data = self.get_current_profile();
                    let src_path = data.src_path.clone();
                    let dst_path = data.dst_path.clone();
                    let copy_screenshot = data.screenshots;
                    drop(data);

                    self.start_backup(src_path, dst_path, copy_screenshot, false);
                }
            },
            backup::BackupState::Busy => {},
            backup::BackupState::Finished => {
                self.finish_up_backup();
            }
        }
    }

    fn timer_tick(&self) {
        let changes = read_rwlock_or(&WATCHER_HAS_CHANGES, false);
        if changes {
            self.tick_backup(self.tick_screenshot());
        }

        self.timer.start();
    }

    fn refresh_backup_list(&self) {
        let profile = self.get_current_profile();
        let dst_path = profile.dst_path.clone();

        if dst_path.len() == 0 {
            self.savegame_list.clear_list(true);
            return;
        }

        match backup::look_for_backups(&dst_path) {
            Ok(backups) => {
                self.savegame_list.set_redraw(false);
                self.savegame_list.clear_list(false);
                for backup in backups {
                    self.savegame_list.push_savegame(backup);
                }

                self.savegame_list.update_list(false);
                self.savegame_list.set_redraw(true);

                let src_path = profile.src_path.clone();

                let mut found_current_backup = false;
                let live_hashes = backup::create_hash_list(&src_path);
                for (i, backup) in (&*self.savegame_list.data.borrow()).iter().enumerate() {
                    if !!backup::hash_list_cmp(&live_hashes, &backup.checksums) {
                        self.savegame_list.check_row(i);
                        found_current_backup = true;
                        break;
                    }
                }

                if !found_current_backup {
                    self.start_backup(src_path, dst_path, false, false);
                    self.finish_up_backup();
                }

            },
            Err(err) => {
                nwg::modal_error_message(&self.window, "Backup error", format!("Error reading backups: {:?}", err).as_str());
            }
        }
    }

    fn load_data(&self) {
        self.tooltip.register_callback(&self.profile_add);
        self.tooltip.register_callback(&self.profile_rename);
        self.tooltip.register_callback(&self.profile_remove);

        self.autosave_interval_unit.set_collection(vec![ProfileIntervalUnit::Seconds, ProfileIntervalUnit::Minutes, ProfileIntervalUnit::Hours]);

        let mut profiles: Vec<SavegameManagerProfile> = match File::open(DATA_FILE) {
            Ok(file) => {
                match serde_json::from_reader::<File, Vec<SavegameManagerProfile>>(file) {
                    Ok(json) => {
                        json
                    },
                    Err(err) => {
                        nwg::modal_error_message(&self.window, "Config file error", format!("Unable to parse config file. {:?}", err).as_str());
                        vec![]
                    }
                }
            },
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::NotFound => { println!("No config file found"); },
                    std::io::ErrorKind::PermissionDenied => { nwg::modal_error_message(&self.window, "Config file error", "Unable to open config file. Permission was denied."); },
                    e => { nwg::modal_error_message(&self.window, "Config file error", format!("An unusual error occured trying to open config file. {:?}", e).as_str()); },
                }
                vec![]
            }
        };

        if profiles.len() == 0 {
            let mut p: SavegameManagerProfile = Default::default();
            p.name = "Default".to_owned();
            p.selected = true;
            profiles.push(p);
        }

        let mut selected_index: Option<usize> = None;
        for (i, profile) in profiles.iter().enumerate() {
            if profile.selected {
                selected_index = Some(i);
                break;
            }
        }

        self.profile_select.set_collection(profiles);
        self.profile_select.set_selection(selected_index);
        self.profile_select_change();
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

                        let mut profile = self.get_current_profile_mut();
                        if button == &self.source_button {
                            profile.src_path = path_string;
                        } else {
                            profile.dst_path = path_string;
                        }
                        drop(profile);
                        *self.profiles_changed.borrow_mut() = true;

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

    fn screenshots_checkbox_click(&self) {
        let mut profile = self.get_current_profile_mut();
        profile.screenshots = match self.screenshots_check.check_state() {
            nwg::CheckBoxState::Unchecked => false,
            _ => true,
        };
        *self.profiles_changed.borrow_mut() = true;
    }

    fn manual_save_checkbox_click(&self) {
        let mut profile = self.get_current_profile_mut();
        profile.manual_save_detection = match self.manual_save_detection_check.check_state() {
            nwg::CheckBoxState::Unchecked => false,
            _ => true,
        };
        *self.profiles_changed.borrow_mut() = true;
    }

    fn show_details(&self) {
        let last_backup = self.selected_backup.borrow();

        match self.savegame_list.get_selected_savegame() {
            Some(savegame) => {
                if let Some(last) = last_backup.as_ref() {
                    if *last == savegame.name {
                        return;
                    }
                }
                drop(last_backup);
                *self.selected_backup.borrow_mut() = Some(savegame.name.clone());

                self.savegame_detail_name_content.set_text(savegame.name.as_str());
                self.savegame_detail_date_content.set_text(local_datetime_from_millis(savegame.date).format("%c").to_string().as_str());
                self.savegame_detail_checksums_content.set_text(savegame.checksums.iter().map(|c| {
                    let file_name = if c.0.len() > 21 { format!("{}…", &c.0[..20]) } else { format!("{}", c.0) };
                    String::from(&format!("{}… / {}", &c.1[..15], file_name))
                }).collect::<Vec<String>>().join("\r\n").as_str());

                let dst_path = PathBuf::from(self.get_current_profile().dst_path.clone()).join(&savegame.name).join("screenshot.jpg");
                if dst_path.exists() && dst_path.is_file() {
                    let mut screenshot = nwg::Bitmap::default();
                    nwg::Bitmap::builder()
                        .source_file(Some(dst_path.to_str().unwrap_or_default()))
                        .size(Some((295, 166)))
                        .strict(false)
                        .build(&mut screenshot).unwrap_or_default();
                    self.savegame_detail_screenshot.set_bitmap(Some(&screenshot));
                } else {
                    self.savegame_detail_screenshot.set_bitmap(Some(&self.no_screenshot));
                }

                self.savegame_load.set_enabled(true);
                self.savegame_rename.set_enabled(true);
                self.savegame_delete.set_enabled(true);
            },
            None => {
                if last_backup.is_none() {
                    return;
                }
                drop(last_backup);
                *self.selected_backup.borrow_mut() = None;

                self.savegame_detail_name_content.set_text("-");
                self.savegame_detail_date_content.set_text("-");
                self.savegame_detail_checksums_content.set_text("-");
                self.savegame_detail_screenshot.set_bitmap(Some(&self.no_screenshot));

                self.savegame_load.set_enabled(false);
                self.savegame_rename.set_enabled(false);
                self.savegame_delete.set_enabled(false);

                self.rename_dialog.set_visible(false);
            }
        }
    }

    fn open_screenshot(&self) {
        if let Some(savegame) = self.savegame_list.get_selected_savegame() {
            let dst_path = PathBuf::from(self.get_current_profile().dst_path.clone()).join(&savegame.name).join("screenshot.jpg");
            if dst_path.exists() && dst_path.is_file() {
                let _ = opener::open(dst_path);
            }
        }
    }

    fn key_released(&self, event: &nwg::EventData) {
        match event {
            nwg::EventData::OnKey(nwg::keys::F5) => {
                self.refresh_backup_list();
            },
            nwg::EventData::OnKey(nwg::keys::F2) => {
                self.rename_click();
            }
            nwg::EventData::OnKey(nwg::keys::DELETE) => {
                self.delete_click();
            }
            _ => {}
        }
    }

    fn load_click(&self) {
        if let Some(savegame) = self.savegame_list.get_selected_savegame() {
            write_to_rwlock(&WATCHER_PAUSED, true);
            let data = self.get_current_profile();
            let src_path = data.src_path.clone();
            let dst_path = data.dst_path.clone();
            
            if let Err(err) = backup::load_backup(&src_path, &dst_path, &savegame) {
                println!("Error loading backup: {:?}", err);
                nwg::modal_error_message(&self.window, "Load error", format!("Error loading backup: {}", err).as_str());
            }

            self.refresh_backup_list();
            self.savegame_list.select_by_name(savegame.name.as_str());
            write_to_rwlock(&WATCHER_PAUSED, false);
        }
    }

    fn rename_click(&self) {
        if let Some(savegame) = self.savegame_list.get_selected_savegame() {
            let (x, y) = self.window.position();
            let (width, height) = self.window.size();
            let (dialog_width, dialog_height) = self.rename_dialog.size();

            *self.rename_mode.borrow_mut() = RenameMode::Backup;
            self.rename_input.set_text(savegame.name.as_str());
            self.rename_dialog.set_position(x + (width as i32 - dialog_width as i32) - 15, y + (height as i32 - dialog_height as i32) - 50);
            self.rename_dialog.set_text("Rename backup");
            self.rename_dialog.set_visible(true);
            self.rename_input.set_selection(0..savegame.name.len() as u32);
            self.rename_input.set_focus();
        }
    }

    fn rename_confirm(&self) {
        match *self.rename_mode.borrow() {
            RenameMode::Backup => {
                if let Some(savegame) = self.savegame_list.get_selected_savegame() {
                    let new_name: String = self.rename_input.text()
                        .replace("<", "").replace(">", "")
                        .replace(":", "").replace("\"", "")
                        .replace("/", "").replace("\\", "")
                        .replace("|", "").replace("?", "")
                        .replace("*", "").trim().to_owned();
        
                    if new_name.len() > 0 {
                        match backup::rename_backup(&self.get_current_profile().dst_path, &savegame.name, &new_name) {
                            Ok(_) => {
                                self.rename_dialog.set_visible(false);
                                self.refresh_backup_list();
                                self.savegame_list.select_by_name(new_name.as_str());
                            },
                            Err(err) => {
                                println!("Error renaming backup: {:?}", err);
                                nwg::modal_error_message(&self.rename_dialog, "Rename error", format!("Error renaming backup: {}", err).as_str());
                            }
                        }
                    } else {
                        nwg::modal_error_message(&self.rename_dialog, "Rename error", "The new name must not be empty");
                    }
                }
            },
            RenameMode::Profile => {
                let new_name = self.rename_input.text();
                if new_name.len() > 0 {
                    let mut profile = self.get_current_profile_mut();
                    profile.name = new_name;
                    drop(profile);

                    let selection = self.profile_select.selection();

                    self.profile_select.sync();
                    self.profile_select.set_selection(selection);

                    *self.profiles_changed.borrow_mut() = true;
                    self.rename_dialog.set_visible(false);
                }
            },
        }
    }

    fn rename_cancel(&self, event: nwg::Event) {
        match event {
            nwg::Event::OnButtonClick => {
                self.rename_dialog.set_visible(false);
            },
            nwg::Event::OnKeyEsc => {
                self.rename_dialog.set_visible(false);
            },
            _ => {}
        }
    }

    fn delete_click(&self) {
        if let Some(savegame) = self.savegame_list.get_selected_savegame() {
            let result = nwg::modal_message(&self.window, &nwg::MessageParams { title: "Deleting backup", content: format!("Are you sure you want to delete {}?\n(We'll just move it to the recycle bin for your.)", savegame.name).as_str(), buttons: nwg::MessageButtons::YesNo, icons: nwg::MessageIcons::Question });
            match result {
                nwg::MessageChoice::Yes => {
                    match crate::backup::recycle_backup(&self.get_current_profile().dst_path, &savegame.name) {
                        Ok(_) => {
                            self.refresh_backup_list();
                        },
                        Err(err) => {
                            println!("Error deleting backup: {:?}", err);
                            nwg::modal_error_message(&self.window, "Delete error", format!("Error deleting backup: {}", err).as_str());
                        }
                    }
                },
                _ => {}
            }
        }
    }

    fn profile_select_change(&self) {
        let mut profiles = self.profile_select.collection_mut();
        for profile in &mut *profiles {
            profile.selected = false;
        }

        let selection = self.profile_select.selection();

        let mut autosave_amount = String::new();
        let mut autosave_interval = String::new();
        let mut autosave_interval_unit = ProfileIntervalUnit::Minutes;
        if let Some(selection) = selection {
            profiles[selection].selected = true;

            self.source_button.set_text(if profiles[selection].src_path.len() > 0 { profiles[selection].src_path.as_str() } else { "Select source path" });
            self.dest_button.set_text(if profiles[selection].dst_path.len() > 0 { profiles[selection].dst_path.as_str() } else { "Select backup path" });
            self.screenshots_check.set_check_state(if profiles[selection].screenshots { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked });
            self.manual_save_detection_check.set_check_state(if profiles[selection].manual_save_detection { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked });

            autosave_amount = format!("{}", profiles[selection].auto_saves_max);
            autosave_interval = format!("{}", profiles[selection].auto_saves_interval);
            autosave_interval_unit = profiles[selection].auto_saves_interval_unit.clone();
        }
        drop(profiles);

        self.autosave_amount.set_text(autosave_amount.as_str());
        self.autosave_interval.set_text(autosave_interval.as_str());
        self.autosave_interval_unit.set_selection(Some(match autosave_interval_unit {
            ProfileIntervalUnit::Seconds => 0,
            ProfileIntervalUnit::Minutes => 1,
            ProfileIntervalUnit::Hours => 2,
        }));

        *self.profiles_changed.borrow_mut() = true;
        self.rename_dialog.set_visible(false);
        self.start_watcher();
        self.refresh_backup_list();
    }

    fn tooltip_text(&self, evt: nwg::Event, evt_data: &nwg::EventData, handle: &nwg::ControlHandle) {
        match evt {
            nwg::Event::OnTooltipText => {
                let tooltip = if handle == &self.profile_add {
                    "Add new profile"
                } else if handle == &self.profile_rename {
                    "Rename selected profile"
                } else if handle == &self.profile_remove {
                    "Remove selected profile"
                } else {
                    ""
                };
                if tooltip.len() > 0 {
                    let tooltip_data = evt_data.on_tooltip_text();
                    tooltip_data.set_text(tooltip);
                }
            },
            _ => {}
        }
    }

    fn profile_add(&self) {
        let mut new_profile = SavegameManagerProfile::default();
        new_profile.name = "New profile".to_owned();
        self.profile_select.push(new_profile);

        self.profile_select.set_selection(Some(self.profile_select.collection().len() - 1));
        self.profile_select_change();
    }

    fn profile_rename(&self) {
        let profile = self.get_current_profile();

        let (x, y) = self.window.position();
        let (width, _) = self.window.size();
        let (dialog_width, _) = self.rename_dialog.size();

        *self.rename_mode.borrow_mut() = RenameMode::Profile;
        self.rename_input.set_text(profile.name.as_str());
        self.rename_dialog.set_position(x + (width as i32 - dialog_width as i32) - 15, y + 50);
        self.rename_dialog.set_text("Rename profile");
        self.rename_dialog.set_visible(true);
        self.rename_input.set_selection(0..profile.name.len() as u32);
        self.rename_input.set_focus();
    }

    fn profile_remove(&self) {
        if self.profile_select.collection().len() == 1 {
            nwg::modal_info_message(&self.window, "Sorry Dave, I'm afraid I can't do that.", "You need at least one profile.");
            return;
        }

        let result = nwg::modal_message(&self.window, &nwg::MessageParams { title: "Deleting profile", content: "Are you sure you want to delete the selected profile?", buttons: nwg::MessageButtons::YesNo, icons: nwg::MessageIcons::Question });
        match result {
            nwg::MessageChoice::Yes => {
                let selection = self.profile_select.selection();
                if let Some(selection) = selection {
                    self.profile_select.remove(selection);

                    self.profile_select.set_selection(Some(0));
                    self.profile_select_change();
                }
            },
            _ => {}
        }
    }

    fn autosave_text_input(&self, handle: &nwg::ControlHandle) {
        let mut profile = self.get_current_profile_mut();
        if handle == &self.autosave_amount {
            profile.auto_saves_max = self.autosave_amount.text().parse().unwrap_or(0);
        } else if handle == &self.autosave_interval {
            profile.auto_saves_interval = self.autosave_interval.text().parse().unwrap_or(0);
        }
        *self.profiles_changed.borrow_mut() = true;
    }

    fn interval_unit_select_change(&self) {
        let mut profile = self.get_current_profile_mut();
        let collection = self.autosave_interval_unit.collection();
        profile.auto_saves_interval_unit = collection[self.autosave_interval_unit.selection().unwrap_or(0)].clone();
        *self.profiles_changed.borrow_mut() = true;
    }
}


#[derive(Default)]
struct SavegameListView {
    base: nwg::ListView,
    image_list: RefCell<nwg::ImageList>,
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

    fn get_latest_non_temp(&self) -> Option<SavegameMeta> {
        let data = self.data.borrow();
        for savegame in data.iter() {
            if !savegame.is_temp() {
                return Some(savegame.clone());
            }
        }
        None
    }

    fn prepare_list(&self) {
        let mut image_list = self.image_list.borrow_mut();
        nwg::ImageList::builder()
            .size((9, 9))
            .initial(3)
            .grow(0)
            .build(&mut image_list).expect("Failed to create image list");

        image_list.add_bitmap(&nwg::Bitmap::from_bin(include_bytes!("../assets/empty.bmp")).unwrap_or_default());
        image_list.add_bitmap(&nwg::Bitmap::from_bin(include_bytes!("../assets/unchecked.bmp")).unwrap_or_default());
        image_list.add_bitmap(&nwg::Bitmap::from_bin(include_bytes!("../assets/checked.bmp")).unwrap_or_default());

        self.set_image_list(Some(&image_list), nwg::ListViewImageListType::Small);

        self.insert_column(nwg::InsertListViewColumn { index: Some(0), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(300), text: Some("Name".to_owned()) });
        self.insert_column(nwg::InsertListViewColumn { index: Some(1), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(300), text: Some("Date".to_owned()) });
    }

    fn clear_list(&self, disable_redraw: bool) {
        if disable_redraw {
            self.set_redraw(false);
        }

        let mut data = self.data.borrow_mut();
        data.clear();
        self.clear();

        if disable_redraw {
            self.set_redraw(true);
        }
    }

    fn push_savegame(&self, meta: SavegameMeta) {
        let mut data = self.data.borrow_mut();
        let index = data.len();
        data.push(meta.clone());
        drop(data);

        let save_timestamp = local_datetime_from_millis(meta.date);

        let row = [
            nwg::InsertListViewItem { column_index: 0, index: Some(index as i32), text: Some(meta.name), image: Some(1) },
            nwg::InsertListViewItem { column_index: 1, index: Some(index as i32), text: Some(save_timestamp.format("%c").to_string()), image: None },
        ];

        self.insert_item(row[0].clone());
        self.update_item(index, row[1].clone());
    }

    fn update_list(&self, disable_redraw: bool) {
        let mut data = self.data.borrow_mut();
        data.sort_by(|a, b| b.date.cmp(&a.date));
        drop(data);

        let data = self.data.borrow();

        if disable_redraw {
            self.set_redraw(false);
        }

        let mut diff: i32 = data.len() as i32 - self.len() as i32;
        while diff != 0 {
            if diff > 0 {
                self.insert_item(nwg::InsertListViewItem { column_index: 0, index: None, text: None, image: None });
                diff -= 1;
            } else if diff < 0 {
                self.remove_item(self.len());
                diff += 1;
            }
        }

        let mut index = 0;
        for row in &*data {
            let save_timestamp = local_datetime_from_millis(row.date);

            self.update_item(index, nwg::InsertListViewItem { column_index: 0, index: Some(index as i32), text: Some(row.name.clone()), image: Some(1) });
            self.update_item(index, nwg::InsertListViewItem { column_index: 1, index: Some(index as i32), text: Some(save_timestamp.format("%c").to_string()), image: None });

            index += 1;
        }

        if disable_redraw {
            self.set_redraw(true);
        }
    }

    fn get_selected_savegame(&self) -> Option<SavegameMeta> {
        if let Some(index) = self.selected_item() {
            let data = self.data.borrow();
            Some(data[index as usize].clone())
        } else {
            None
        }
    }

    fn check_row(&self, row: usize) {
        let data = self.data.borrow();
        for (i, savegame) in data.iter().enumerate() {
            self.update_item(i, nwg::InsertListViewItem { column_index: 0, index: Some(i as i32), text: Some(savegame.name.clone()), image: Some(if i == row { 2 } else { 1 }) });
        }
    }

    fn select_by_name(&self, name: &str) {
        let data = self.data.borrow();
        for (i, savegame) in data.iter().enumerate() {
            if savegame.name == name {
                self.select_item(i, true);
                self.set_focus();
                break;
            }
        }
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
        list.set_headers_enabled(true);
        Ok(())
    }
}

pub fn start_app() {
    nwg::init().expect("Failed to init native Windows gui");

    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .family("Segoe UI")
        .size(16)
        .weight(400)
        .build(&mut font).expect("Failed to build default font");
    nwg::Font::set_global_default(Some(font));
    let app = SavegameManagerApp::build_ui(Default::default()).expect("Failed to build ui");
    app.load_data();
    app.start_watcher();

    // make window visible after construction is done to avoid render glitches
    app.window.set_visible(true);
    nwg::dispatch_thread_events();
}