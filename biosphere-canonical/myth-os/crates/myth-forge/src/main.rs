mod app;
mod canvas;
mod export;
mod inspector;
mod library;
mod scene;
mod theme;

use app::ForgeApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MYTH-FORGE  —  Instrument Builder")
            .with_inner_size([1400.0, 860.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "myth-forge",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(ForgeApp::default()))
        }),
    )
}
