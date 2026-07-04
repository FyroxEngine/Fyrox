mod app;
mod channel;
mod master;
mod state;
mod theme;
mod widgets;

use app::ControllerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("THE AXIOM CONTROLLER")
            .with_inner_size([1440.0, 820.0])
            .with_min_inner_size([1100.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "myth-controller",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(ControllerApp::default()))
        }),
    )
}
