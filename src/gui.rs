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

    // #[nwg_partial(parent: source_frame)]
    // #[nwg_events((button, OnButtonClick): [SavegameManagerApp::select_folder(SELF, "Select source folder", FolderSelectRow::Source)])]
    // source: FolderSelectRowPartial,

    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(25.0) })]
    destination_frame: nwg::Frame,

    #[nwg_partial(parent: destination_frame)]
    // #[nwg_events((button, OnButtonClick): [SavegameManagerApp::select_folder(self, "Select destination folder", FolderSelectRow::Destination)])]
    destination: FolderSelectRowPartial,

    #[nwg_control(parent: window, text: "", background_color: Some([0,0,0]))]
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

    fn select_folder(&self, button: &nwg::Button) {
        if let Ok(dialog) = SavegameManagerApp::open_dialog(title, &self.window.handle) {
            match dialog.get_selected_item() {
                Ok(path) =>
                    if let Ok(path_string) =  path.into_string() {
                        // match row {
                        //     FolderSelectRow::Source => &self.source.button,
                        //     FolderSelectRow::Destination => &self.destination.button
                        // }
                        // .set_text(path_string.as_str());
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
            source_frame: Default::default(), source_layout: Default::default(), source_label: Default::default(), source_button: Default::default(),
            destination_frame: Default::default(), destination: Default::default(),
            lbl_placeholder: Default::default()
        };

        // app.source.init_label_text = "Source:";
        // app.source.init_button_text = "Select source folder";
        app.destination.init_label_text = "Destination:";
        app.destination.init_button_text = "Select destination folder";

        app
    }
}

#[derive(Default, NwgPartial)]
pub struct FolderSelectRowPartial {
    init_label_text: &'static str,
    init_button_text: &'static str,

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