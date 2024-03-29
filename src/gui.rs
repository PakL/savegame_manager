use native_windows_gui as nwg;
use native_windows_derive as nwd;

use nwd::{NwgPartial, NwgUi};
use nwg::NativeUi;

use nwg::stretch::{ geometry::{ Size, Rect }, style:: { Dimension as D, FlexDirection } };

const NO_PADDING: Rect<D> = Rect { start: D::Points(0.0), end: D::Points(0.0), top: D::Points(0.0), bottom: D::Points(0.0) };

#[derive(Default, NwgUi)]
pub struct SavegameManagerApp {
    // source_folder: String,

    #[nwg_control(size: (800, 600), title: "Savegame Manager", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [nwg::stop_thread_dispatch()] )]
    window: nwg::Window,
    
    #[nwg_layout(parent: window, flex_direction: FlexDirection::Column)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Points(30.0) })]
    source_frame: nwg::Frame,

    #[nwg_partial(parent: source_frame)]
    source: SourceRow,


    #[nwg_control(text: "", background_color: Some([0,0,0]))]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    lbl_placeholder: nwg::Label,
}

#[derive(Default, NwgPartial)]
pub struct SourceRow {
    #[nwg_layout(flex_direction: FlexDirection::Row, padding: NO_PADDING)]
    layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Source:")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Points(100.0), height: D::Auto })]
    lbl_source: nwg::Label,
    #[nwg_control(text: "Select source folder")]
    #[nwg_layout_item(layout: layout, size: Size { width: D::Auto, height: D::Auto }, flex_grow: 1.0)]
    btn_source: nwg::Button,

}

pub fn start_app() {
    nwg::init().expect("Failed to init native Windows gui");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");
    let _ui = SavegameManagerApp::build_ui(Default::default()).expect("Failed to build ui");
    nwg::dispatch_thread_events();
}