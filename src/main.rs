#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor;
mod filetree;
mod highlighter;
mod lua_engine;
mod plugin;
mod theme;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("kittywrite")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "kittywrite",
        options,
        Box::new(|cc| Ok(Box::new(app::KittyWriteApp::new(cc)))),
    )
}
