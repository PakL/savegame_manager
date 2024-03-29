use native_windows_gui as nwg;
use native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;

use nwg::stretch::{ geometry::{ Size, Rect }, style:: { Dimension as D, FlexDirection } };

const NO_PADDING: Rect<D> = Rect { start: D::Points(0.0), end: D::Points(0.0), top: D::Points(0.0), bottom: D::Points(0.0) };

#[derive(Default, NwgUi)]
pub struct SavegameManagerApp {
    #[nwg_control(size: (800, 600), title: "Savegame Manager", flags: "MAIN_WINDOW")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,

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


    // Placeholder
    #[nwg_control(parent: window)]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    savegame_list: SavegameListView,
}

impl SavegameManagerApp {
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
        let title = if button == &self.source_button { "Select source folder" } else { "Select destination folder" };

        if let Ok(dialog) = SavegameManagerApp::open_dialog(title, &self.window.handle) {
            match dialog.get_selected_item() {
                Ok(path) =>
                    if let Ok(path_string) =  path.into_string() {
                        button.set_text(path_string.as_str());
                    }
                Err(_) => {}
            }
        }
    }
}

use std::cell::RefCell;
use serde::{Deserialize, Serialize};
use time;

#[derive(Clone, Default, Serialize, Deserialize)]
struct SavegameMeta {
    name: String,
    date: i64,
    checksums: Vec<(String, String)>,

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
                .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        }
    }

    fn prepare_list(&self) {
        self.base.insert_column(nwg::InsertListViewColumn { index: Some(0), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(100), text: Some("Name".to_owned()) });
        self.base.insert_column(nwg::InsertListViewColumn { index: Some(1), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(100), text: Some("Date".to_owned()) });
        self.base.insert_column(nwg::InsertListViewColumn { index: Some(2), fmt: Some(nwg::ListViewColumnFlags::LEFT), width: Some(100), text: Some("Checksum".to_owned()) });

        let row = [
            nwg::InsertListViewItem { column_index: 0, index: Some(0), text: Some("Name".to_owned()), image: None },
            nwg::InsertListViewItem { column_index: 1, index: Some(0), text: Some("Date".to_owned()), image: None },
            nwg::InsertListViewItem { column_index: 2, index: Some(0), text: Some("Checksum".to_owned()), image: None },
        ];
        self.base.insert_items(&row);
    }

    fn add_savegame(&self, meta: SavegameMeta) {
        let mut data = self.data.borrow_mut();
        data.push(meta.clone());
        
        let index: i32 = data.len() as i32;

        let save_timestamp = time::OffsetDateTime::from_unix_timestamp(meta.date).unwrap_or(time::OffsetDateTime::now_utc());
        let row = [
            nwg::InsertListViewItem { column_index: 0, index: Some(index), text: Some(meta.name), image: None },
            nwg::InsertListViewItem { column_index: 1, index: Some(index), text: Some(save_timestamp.format(&time::format_description::well_known::Rfc2822).unwrap_or_default()), image: None },
            nwg::InsertListViewItem { column_index: 2, index: Some(index), text: Some(meta.checksums.iter().map(|cs| format!("{}: {}", cs.0, cs.1)).collect::<Vec<String>>().join(", ")), image: None },
        ];
        self.base.insert_items(&row);
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
    let _app = SavegameManagerApp::build_ui(Default::default()).expect("Failed to build ui");

    // make window visible after construction is done to avoid render glitches
    _app.window.set_visible(true);
    nwg::dispatch_thread_events();
}