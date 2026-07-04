use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppScreen {
    #[default]
    Splash,
    Landing,
    VaultView,
    VaultSetup,
    /// Cinematic world-creation sequence played after INITIATE GENESIS.
    /// Full-screen — the shell does not render during this state.
    GenesisBootSequence,
    /// Fallback state shown when the app encounters an unrecoverable
    /// state transition or missing resource. Prevents a blank window.
    Error,
}
