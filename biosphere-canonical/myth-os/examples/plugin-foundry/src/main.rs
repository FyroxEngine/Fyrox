use eframe::NativeOptions;
use plugin_foundry::app::FoundryApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Plugin Foundry — myth-os")
            .with_inner_size([900.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Plugin Foundry",
        options,
        Box::new(|cc| {
            Ok(Box::new(
                FoundryApp::new(cc).expect("Foundry init failed")
            ))
        }),
    )
}
