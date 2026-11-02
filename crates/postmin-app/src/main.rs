#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod dialogs;
mod environment_view;
mod highlight;
mod request_view;
mod response_view;
mod sidebar;
mod theme;
mod widgets;

use eframe::egui::{self, IconData};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_title("PostMin")
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "PostMin",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Box::new(app::PostMinApp::new(cc))
        }),
    )
}

fn app_icon() -> IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon_128.png"))
        .expect("bundled icon_128.png is a valid PNG")
}
