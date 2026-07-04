mod core_status;
mod plugin_registry;
mod state;
mod theatre_state;
mod ui;
mod vault_store;

use bevy::{log::LogPlugin, prelude::*};
use bevy_egui::EguiPlugin;
use state::AppScreen;
use core_status::CoreStatusPlugin;
use ui::{
    UiSet,
    error_screen::ErrorScreenPlugin,
    genesis_boot::GenesisBootPlugin,
    landing::LandingPlugin,
    shell::ShellPlugin,
    splash::SplashPlugin,
    vault_setup::VaultSetupPlugin,
    vault_view::VaultViewPlugin,
};
use plugin_registry::PluginRegistryPlugin;
use theatre_state::TheatreStatePlugin;
use vault_store::VaultStorePlugin;

fn main() {
    // --dry-run: validate the vault store and plugin registry, then exit.
    // Useful for CI / headless smoke tests.
    if std::env::args().any(|a| a == "--dry-run") {
        let store = vault_store::VaultStore::load();
        println!("[dry-run] VaultStore OK — {} vault(s) loaded", store.vaults.len());
        let registry = plugin_registry::PluginRegistry { plugins: plugin_registry::CORE_PLUGINS };
        println!("[dry-run] PluginRegistry OK — {} plugin(s) available", registry.plugins.len());
        println!("[dry-run] All systems nominal. Exiting.");
        return;
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title:      "MYTHOS  ·  Quantum Ecosystem".into(),
                        resolution: (1400.0, 900.0).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "quantum_vault=debug,bevy_render=warn,wgpu=error,egui=warn".into(),
                    level:  bevy::log::Level::DEBUG,
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin)
        .init_state::<AppScreen>()
        // ── System ordering: shell panels before page content ─────────────────
        .configure_sets(Update, UiSet::Shell.before(UiSet::Page))
        // ── Data ──────────────────────────────────────────────────────────────
        .add_plugins(VaultStorePlugin)
        .add_plugins(PluginRegistryPlugin)
        .add_plugins(CoreStatusPlugin)
        .add_plugins(TheatreStatePlugin)
        // ── UI layers ─────────────────────────────────────────────────────────
        .add_plugins(SplashPlugin)       // Splash state
        .add_plugins(ShellPlugin)        // Outer ring + state-routed inner ring
        .add_plugins(LandingPlugin)      // Vault card grid
        .add_plugins(VaultViewPlugin)    // Vault interior
        .add_plugins(VaultSetupPlugin)   // New-vault wizard
        .add_plugins(GenesisBootPlugin)  // Cinematic world-init sequence
        .add_plugins(ErrorScreenPlugin)  // Fallback error state
        .run();
}
