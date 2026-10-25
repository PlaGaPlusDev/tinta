//! tinta — vault de notas Markdown nativo en Rust (egui), libre de Electron.

mod app;
mod commands;
mod editor;
mod fuzzy;
mod graph;
mod index;
mod search;
mod switcher;
mod theme;
mod vault;
mod watcher;

use app::TintaApp;

fn main() -> eframe::Result {
    let vault_path = std::env::args()
        .nth(1)
        .or_else(vault::load_last_vault);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("tinta")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "tinta",
        options,
        Box::new(move |cc| {
            crate::theme::configure(&cc.egui_ctx, true);
            Ok(Box::new(TintaApp::new(vault_path)))
        }),
    )
}
