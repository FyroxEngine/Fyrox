pub mod error_screen;
pub mod genesis_boot;
pub mod landing;
pub mod shell;
pub mod splash;
pub mod theme;
pub mod vault_setup;
pub mod vault_view;

use bevy::prelude::*;

/// Shell panels must be flushed to egui before any page content is added.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UiSet {
    Shell,
    Page,
}
