use native_windows_gui as nwg;
use native_windows_derive as nwd;

use nwd::{NwgPartial, NwgUi};
use nwg::NativeUi;

use nwg::stretch::{ geometry::{ Size, Rect }, style:: { Dimension as D, FlexDirection } };

const NO_PADDING: Rect<D> = Rect { start: D::Points(0.0), end: D::Points(0.0), top: D::Points(0.0), bottom: D::Points(0.0) };

#[derive(NwgUi)]
pub struct SavegameManagerApp {
    // source_folder: String,

    #[nwg_control(size: (800, 600), title: "Savegame Manager", flags: "MAIN_WINDOW")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,
    
    #[nwg_layout(parent: window, flex_direction: FlexDirection::Column)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    source_frame: nwg::Frame,

    #[nwg_partial(parent: source_frame)]
    #[nwg_events((button, OnButtonClick): [SavegameManagerApp::select_source_folder])]
    source: FolderSelectRow,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    destination_frame: nwg::Frame,

    #[nwg_partial(parent: destination_frame)]
    #[nwg_events((button, OnButtonClick): [SavegameManagerApp::select_dest_folder])]
    destination: FolderSelectRow,

    #[nwg_control(text: "", background_color: Some([0,0,0]))]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    lbl_placeholder: nwg::Label,
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

    fn select_source_folder(&self) {
        if let Ok(dialog) = SavegameManagerApp::open_dialog("Select source folder", &self.window.handle) {
            match dialog.get_selected_item() {
                Ok(path) =>
                    if let Ok(path_string) =  path.into_string() {
                        self.source.button.set_text(path_string.as_str());
                    }
                Err(_) => {}
            }
        }
    }

    fn select_dest_folder(&self) {
        if let Ok(dialog) = SavegameManagerApp::open_dialog("Select destination folder", &self.window.handle) {
            match dialog.get_selected_item() {
                Ok(path) =>
                    if let Ok(path_string) =  path.into_string() {
                        self.destination.button.set_text(path_string.as_str());
                    }
                Err(_) => {}
            }
        }
    }
}

impl Default for SavegameManagerApp {
    fn default() -> Self {
        let mut app = Self {
            window: Default::default(), layout: Default::default(),
            source_frame: Default::default(), source: Default::default(),
            destination_frame: Default::default(), destination: Default::default(),
            lbl_placeholder: Default::default()
        };

        app.source.init_label_text = "Source:";
        app.source.init_button_text = "Select source folder";
        app.destination.init_label_text = "Destination:";
        app.destination.init_button_text = "Select destination folder";

        app
    }
}

#[derive(Default, NwgPartial)]
pub struct FolderSelectRow {
    init_label_text: &'static str,
    init_button_text: &'static str,

    #[nwg_layout(flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: data.init_label_text)]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Points(100.0), height: D::Auto })]
    label: nwg::Label,
    #[nwg_control(text: data.init_button_text)]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    button: nwg::Button,
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